import socket
import time
import struct
import sys
from datetime import datetime
from dataclasses import dataclass, astuple


def u16(num: int):
    return num.to_bytes(2, "big", signed=False)


def u32(num: int):
    return num.to_bytes(4, "big", signed=False)


@dataclass
class Camera:
    road: int
    mile: int
    limit: int

    def make(self):
        message = bytearray()
        message.extend(bytes.fromhex("80"))
        for n in [self.road, self.mile, self.limit]:
            message.extend(u16(n))
        return message


@dataclass
class Dispatcher:
    roads: list[int]

    def make(self):
        message = bytearray()
        message.extend(bytes.fromhex("81"))
        message.extend(len(self.roads).to_bytes(1, "big", signed=False))
        for road in self.roads:
            message.extend(u16(road))
        return message


@dataclass
class WantHeartBeat:
    interval: int

    def make(self):
        message = bytearray()
        message.extend(bytes.fromhex("40"))
        message.extend(
            u32(self.interval)
        )
        return message


def pstr(s: str) -> list:
    bstr = s.encode()
    data = [len(bstr), *bstr]
    return data


@dataclass
class Plate:
    plate: str
    timestamp: int

    def make(self):
        message = bytearray()
        message.extend(bytes.fromhex("20"))
        message.extend(pstr(self.plate))
        message.extend(u32(self.timestamp))
        return message


def camera_heartbeat(mile, ts):
    """
    1. Sends I am camera message
    2. Sends want heartbeat message 
       for 2 seconds
    3. Waits for heartbeat messages
    """
    c = Camera(123, mile, 60)
    wh = WantHeartBeat(25)
    plate = Plate("UN1X", ts)
    messages = [wh.make(), c.make(), plate.make()]
    return messages, True


def dispatcher_subscribe_and_listen():
    """
    1. Sends I am dispatcher message
    2. Sends want heartbeat message 
       for 2 seconds
    3. Waits for heartbeat messages
    """
    d = Dispatcher([123, 124, 125])
    wh = WantHeartBeat(25)
    messages = [d.make(), wh.make()]
    return messages, True


def send_message(messages, has_read):
    with socket.socket(
            socket.AF_INET, 
            socket.SOCK_STREAM 
    ) as sock:
        sock.connect(("127.0.0.1", 8838))
        for message in messages:
            print("sending", list(message))
            sock.sendall(message)

        if not has_read:
            return

        while True:
            chunk = sock.recv(1024)
            if not chunk:
                print("<moving-on>")
                return
        
            print(">>", int.from_bytes(chunk, 'big'), "<<")


import threading

def main(client_types):
    funcs = {
        "c1": lambda: camera_heartbeat(8, 0),
        "c2": lambda: camera_heartbeat(9, 45),
        "d": lambda: dispatcher_subscribe_and_listen(),
    }
    threads = []

    for client_type in client_types:
        print(f"Starting: {client_type}")
        aaa = funcs[client_type]()
        thread = threading.Thread(target=send_message, args=aaa)
        thread.start()
        threads.append(thread)

    # Wait for all threads to complete
    for thread in threads:
        thread.join()


if __name__ == "__main__":
    if len(sys.argv) >= 2:
        main(sys.argv[1:])
    else:
        print("Type of client required: c/d")
