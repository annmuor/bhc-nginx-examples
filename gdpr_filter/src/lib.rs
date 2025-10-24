use ngx::ffi::{
    NGX_HTTP_MODULE, NGX_OK, memcpy, ngx_buf_t, ngx_chain_t, ngx_command_t, ngx_conf_t,
    ngx_http_module_t, ngx_http_output_body_filter_pt, ngx_http_request_t,
    ngx_http_top_body_filter, ngx_int_t, ngx_module_t, ngx_pcalloc, u_char,
};
use ngx::http::HttpModule;
use ngx::ngx_modules;
use regex::{Captures, Regex, Replacer};
use std::ptr::{addr_of, null_mut};
use std::slice::from_raw_parts;

struct GDPRFilterModule;

impl HttpModule for GDPRFilterModule {
    fn module() -> &'static ngx_module_t {
        unsafe { &*addr_of!(ngx_http_gdpr_module) }
    }
    unsafe extern "C" fn postconfiguration(_cf: *mut ngx_conf_t) -> ngx_int_t {
        unsafe {
            next_filter = ngx_http_top_body_filter;
            ngx_http_top_body_filter = Some(ngx_http_gdpr_body_filter);
        }
        NGX_OK as ngx_int_t
    }
}

#[allow(non_upper_case_globals)]
static mut next_filter: ngx_http_output_body_filter_pt = None;
static mut NGX_HTTP_GDPR_FILTER_COMMANDS: [ngx_command_t; 1] = [ngx_command_t::empty()];

static NGX_HTTP_GDPR_FILTER_CTX: ngx_http_module_t = ngx_http_module_t {
    preconfiguration: None,
    postconfiguration: Some(GDPRFilterModule::postconfiguration),
    create_main_conf: None,
    init_main_conf: None,
    create_srv_conf: None,
    merge_srv_conf: None,
    create_loc_conf: None,
    merge_loc_conf: None,
};

#[allow(non_upper_case_globals)]
#[unsafe(no_mangle)]
#[used]
pub static mut ngx_http_gdpr_module: ngx_module_t = ngx_module_t {
    ctx: addr_of!(NGX_HTTP_GDPR_FILTER_CTX) as _,
    commands: unsafe { &NGX_HTTP_GDPR_FILTER_COMMANDS[0] as *const _ as *mut _ },
    type_: NGX_HTTP_MODULE as _,
    ..ngx_module_t::default()
};

unsafe extern "C" fn ngx_http_gdpr_body_filter(
    r: *mut ngx_http_request_t,
    chain: *mut ngx_chain_t,
) -> ngx_int_t {
    // filter for GDPR related context - change any Name to N*** and any Surname to S****
    let mut next: *mut ngx_chain_t = chain;
    let mut body = Vec::new();
    while !next.is_null() {
        unsafe {
            let start = (*(*next).buf).pos;
            if !start.is_null() {
                let last = (*(*next).buf).last;
                let size = last.offset_from_unsigned(start) * size_of::<u_char>();
                // try to make a string
                body.extend_from_slice(from_raw_parts(start, size));
            }
            next = (*next).next;
        }
        // Safety: we check for null
    }
    let mut out: *mut ngx_chain_t = chain;
    if replace_emails(&mut body) {
        unsafe {
            let mut new_chain = ngx_chain_t {
                buf: ngx_pcalloc((*r).pool, size_of::<ngx_buf_t>()) as *mut _,
                next: null_mut(),
            };
            (*new_chain.buf).start =
                ngx_pcalloc((*r).pool, body.len() * size_of::<u_char>()) as *mut _;
            (*new_chain.buf).pos = (*new_chain.buf).start;
            (*new_chain.buf).end = (*new_chain.buf).start.add(body.len());
            (*new_chain.buf).last = (*new_chain.buf).end;
            memcpy(
                (*new_chain.buf).start as *mut _,
                body.as_mut_ptr() as *mut _,
                (body.len() * size_of::<u_char>()) as u64,
            );
            (*new_chain.buf).set_memory(1);
            out = &mut new_chain as *mut _;
            // now we need to replace content-length
            if !(*r).header_sent() == 1 {
                (*r).headers_out.content_length_n = (body.len() * size_of::<u_char>()) as i64;
            }
        }
    }
    unsafe {
        next_filter
            .map(|filter| filter(r, out))
            .unwrap_or(NGX_OK as ngx_int_t)
    }
}

struct GDPRRepl;
impl Replacer for GDPRRepl {
    fn replace_append(&mut self, caps: &Captures<'_>, dst: &mut String) {
        (0..caps[1].len()).for_each(|_| {dst.push('*')});
        dst.push('@');
        (0..caps[2].len()).for_each(|_| {dst.push('*')});
        dst.push('.');
        (0..caps[3].len()).for_each(|_| {dst.push('*')});
    }
}
fn replace_emails(data: &mut Vec<u_char>) -> bool {
    let str = String::from_utf8_lossy(data);
    let Ok(re) = Regex::new(r"([a-zA-Z0-9._%+-]+)@([a-zA-Z0-9.-]+)\.([a-zA-Z]{2,})") else {
        return false;
    };
    let new = re.replace_all(&str, GDPRRepl);
    if new.ne(&str) {
        *data = Vec::from(new.as_bytes());
        true
    } else {
        false
    }
}
ngx_modules!(ngx_http_gdpr_module);
