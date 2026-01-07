import socket
import time
import keyboard

ip1 = '192.168.0.119'
port1 = 8765
server1 = (ip1, port1)

socket1 = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
socket1.connect(server1)
print('送信準備完了')
line = '2'
while True:
    # 標準入力からデータを取得
    if keyboard.read_key() == 'space':
        
        # サーバに送信
        socket1.send(line.encode("UTF-8"))
        print("送信完了")
        time.sleep(5)



socket1.close()
print('クライアント側終了です')