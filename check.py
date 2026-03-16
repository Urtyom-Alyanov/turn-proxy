import socket
sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
sock.bind(('127.0.0.1', 12346))
while True:
    data, addr = sock.recvfrom(2048)
    sock.sendto(data, addr)