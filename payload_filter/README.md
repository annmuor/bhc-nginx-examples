# NGINX module "GDPR" PoC

This PoC shows how to filter inbound content in order to stop exploits and payloads.

# How it works
This module handles all incoming HTTP requests, reads their bodies and drops the request if DEADBEEF sequence is found somewhere.

# How to run

```bash
# build the image
podman build -t payload_filter -f build/Dockerfile .
# run the image
podman run -ti -p 8080:8080 localhost/payload_filter:latest
# unfiltered
curl 127.0.0.1:8080 --data='hello,world'
# filtered
curl 127.0.0.1:8080 --data='DEADBEEF'
```

![image](image.gif)
# How to play with the code
In the [lib.rs](src/lib.rs) you may find the functions to read the body:

```rust
unsafe extern "C" fn ngx_http_payload_filter_handler(r: *mut ngx_http_request_t) -> ngx_int_t {
    // ...
}

```

Start there and see what happens.