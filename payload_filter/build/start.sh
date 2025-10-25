#!/bin/sh
python3 /server.py &
nginx -c /etc/nginx/nginx.conf
