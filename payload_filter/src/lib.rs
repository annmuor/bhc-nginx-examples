use anyhow::bail;
use ngx::core::Status;
use ngx::ffi::{
    MAP_SHARED, NGX_HTTP_MODULE, NGX_HTTP_SPECIAL_RESPONSE, NGX_LOG_ERR, NGX_LOG_NOTICE,
    NGX_LOG_WARN, PROT_READ, ngx_array_push, ngx_command_t, ngx_conf_t,
    ngx_http_handler_pt, ngx_http_module_t, ngx_http_phases_NGX_HTTP_ACCESS_PHASE,
    ngx_http_read_client_request_body, ngx_http_request_t, ngx_int_t, ngx_module_t,
};
use ngx::http::{HTTPStatus, HttpModule, HttpModuleMainConf, NgxHttpCoreModule};
use ngx::{ngx_conf_log_error, ngx_log_error, ngx_modules};
use std::ptr::{addr_of, null_mut};
use std::slice::from_raw_parts;

struct PayloadFilterModule;

impl HttpModule for PayloadFilterModule {
    fn module() -> &'static ngx_module_t {
        unsafe { &*addr_of!(ngx_http_payload_filter_module) }
    }
    unsafe extern "C" fn postconfiguration(cf: *mut ngx_conf_t) -> ngx_int_t {
        unsafe {
            let cmcf = NgxHttpCoreModule::main_conf_mut(&*cf).expect("http core main conf");
            let h = ngx_array_push(
                &mut cmcf.phases[ngx_http_phases_NGX_HTTP_ACCESS_PHASE as usize].handlers,
            ) as *mut ngx_http_handler_pt;
            if h.is_null() {
                return Status::NGX_ERROR.into();
            }
            // set an Access phase handler
            *h = Some(ngx_http_payload_filter_handler);
            ngx_conf_log_error!(NGX_LOG_NOTICE, cf, "Incoming handler is installed");
        }
        Status::NGX_OK.into()
    }
}

struct BodySegment<'a> {
    data: &'a [u8],
    unmap: bool,
}

impl Drop for BodySegment<'_> {
    fn drop(&mut self) {
        if self.unmap {
            unsafe {
                let size = std::mem::size_of_val(self.data);
                let addr = self.data.as_ptr();
                libc::munmap(addr as *const _ as *mut _, size);
            }
        }
    }
}

fn extract_body(r: &'_ ngx_http_request_t) -> anyhow::Result<Vec<BodySegment<'_>>> {
    let Some(request_body) = (unsafe { r.request_body.as_ref() }) else {
        bail!("Body is null");
    };
    let mut buf_chain = unsafe { &*request_body.bufs };
    let mut result = Vec::new();
    loop {
        if buf_chain.buf.is_null() {
            break;
        }
        let current_buf = unsafe { &*buf_chain.buf };
        let segment = match current_buf.temporary() | current_buf.mmap() | current_buf.memory() {
            1 => {
                let size = unsafe { current_buf.last.byte_offset_from_unsigned(current_buf.pos) };
                BodySegment {
                    data: unsafe { from_raw_parts(current_buf.pos as *const u8, size) },
                    unmap: false,
                }
            }
            _ => {
                if current_buf.file.is_null() {
                    bail!("Body is in file but file is null")
                }
                let file = unsafe { &*current_buf.file };
                let size = (current_buf.file_last - current_buf.file_pos) as usize;
                let mmaped_data = unsafe {
                    libc::mmap(
                        null_mut(),
                        size,
                        PROT_READ as i32,
                        MAP_SHARED as i32,
                        file.fd,
                        0,
                    )
                };
                if mmaped_data == libc::MAP_FAILED {
                    bail!("Mmap failed");
                }
                BodySegment {
                    data: unsafe { from_raw_parts(mmaped_data as *const u8, size) },
                    unmap: true,
                }
            }
        };
        result.push(segment);
        if buf_chain.next.is_null() {
            break;
        } else {
            buf_chain = unsafe { &*buf_chain.next };
        }
    }
    Ok(result)
}
unsafe extern "C" fn http_payload_body_read_handler(_r: *mut ngx_http_request_t) { // do nothing
}

static PATTERN: &[u8] = b"DEADBEEF";
unsafe extern "C" fn ngx_http_payload_filter_handler(r: *mut ngx_http_request_t) -> ngx_int_t {
    let rc = unsafe { ngx_http_read_client_request_body(r, Some(http_payload_body_read_handler)) };

    if rc >= NGX_HTTP_SPECIAL_RESPONSE as isize {
        return rc;
    }
    let request = unsafe { &*r };
    let body_chunks = match extract_body(request) {
        Ok(v) => v,
        Err(e) => {
            ngx_log_error!(
                NGX_LOG_ERR,
                unsafe { *request.connection }.log,
                "Error reading body chunks: {}",
                e
            );
            return Status::NGX_ERROR.into();
        }
    };
    let mut bail = false;
    // Here you can do any level of filtration and even transformation of the request body.
    // This is the easiest example for you to move further
    // Enjoy the nginx hacking!
    for body_chunk in body_chunks {
        if body_chunk
            .data
            .windows(PATTERN.len())
            .any(|w| w.eq(PATTERN))
        {
            bail = true;
        }
    }
    if bail {
        ngx_log_error!(
            NGX_LOG_WARN,
            unsafe { *request.connection }.log,
            "Request is declined because pattern is found"
        );
        Status::from(HTTPStatus::FORBIDDEN).into()
    } else {
        Status::NGX_DECLINED.into()
    }
}

static mut NGX_PAYLOAD_MODULE_COMMANDS: [ngx_command_t; 1] = [ngx_command_t::empty()];
static NGX_PAYLOAD_MODULE_CTX: ngx_http_module_t = ngx_http_module_t {
    preconfiguration: None,
    postconfiguration: Some(PayloadFilterModule::postconfiguration),
    create_main_conf: None,
    init_main_conf: None,
    create_srv_conf: None,
    merge_srv_conf: None,
    create_loc_conf: None,
    merge_loc_conf: None,
};

#[unsafe(no_mangle)]
#[allow(non_upper_case_globals)]
pub static mut ngx_http_payload_filter_module: ngx_module_t = ngx_module_t {
    ctx: addr_of!(NGX_PAYLOAD_MODULE_CTX) as *const _ as *mut _,
    commands: unsafe { &mut NGX_PAYLOAD_MODULE_COMMANDS[0] as *mut _ },
    type_: NGX_HTTP_MODULE as _,
    ..ngx_module_t::default()
};

ngx_modules!(ngx_http_payload_filter_module);
