import socket
import json

def send_resp(s, *args):
    cmd = f"*{len(args)}\r\n"
    for arg in args:
        arg_bytes = arg.encode('utf-8')
        cmd += f"${len(arg_bytes)}\r\n{arg}\r\n"
    s.send(cmd.encode('utf-8'))
    print(s.recv(1024).decode())

config = {
    "/": "file://../stockgo/public/index.html",
    "/style.css": "file://../stockgo/public/style.css",
    "/api/search": "cmd:///Users/kuangtalin/.bun/bin/bun run ../stockgo/cgi/search.ts"
}

s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
s.connect(('localhost', 6379))
send_resp(s, 'PUT', 'web:config', json.dumps(config))
s.close()
