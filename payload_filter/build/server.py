from http.server import BaseHTTPRequestHandler, HTTPServer

class SimpleHandler(BaseHTTPRequestHandler):
    def do_GET(self):
        print(f"Received GET request: {self.path}")
        self.send_response(200)
        self.end_headers()
        self.wfile.write(b"GET response from backend\n")

    def do_POST(self):
        content_length = int(self.headers.get('Content-Length', 0))
        body = self.rfile.read(content_length)
        print(f"Received POST with body: {body}")
        self.send_response(200)
        self.end_headers()
        self.wfile.write(b"POST response from backend\n")

    def do_PUT(self):
        content_length = int(self.headers.get('Content-Length', 0))
        body = self.rfile.read(content_length)
        print(f"Received POST with body: {body}")
        self.send_response(200)
        self.end_headers()
        self.wfile.write(b"POST response from backend\n")

if __name__ == "__main__":
    HTTPServer(("localhost", 8081), SimpleHandler).serve_forever()
