import socket
import time
import struct


def message_hex(msg_type, int1, int2):
    return (
        msg_type.encode().hex()
        + struct.pack(">i", int1).hex()
        + struct.pack(">i", int2).hex()
        # + format(int1, '08x') 
        # + format(int2, '08x')
    )


def send_message(message, has_read):
    with socket.socket(
            socket.AF_INET, 
            socket.SOCK_STREAM 
    ) as sock:
        sock.connect(("127.0.0.1", 8838))
        
        print("sending first")
        sock.sendall(bytes.fromhex(message)[:-2])
        time.sleep(0.5)
        print("Sending second")
        sock.sendall(bytes.fromhex(message)[-2:])

        if has_read:
            print(">>")
            chunk = sock.recv(4)
            if not chunk:
                print("<moving-on>")
                return

            print(int.from_bytes(chunk, 'big'))
            print("<<")



def main():

    while True:
        given = input(">")
        if given == "q":
            break

        messages = given.split("|")
        msg_hex = ""
        has_read = False

        for msg in messages:

            try:
                msg_type, int1, int2 = msg.split(",")
                if has_read is False and msg_type == "Q":
                    has_read = True
            except Exception as e:
                print(e)
                continue
            
            msg_hex += message_hex(
                msg_type, int(int1), int(int2)
            )

        print(msg_hex)
        send_message(msg_hex, has_read)


if __name__ == "__main__":
    main()
