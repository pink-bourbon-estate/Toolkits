from machine import Pin
import time

button = Pin(17, Pin.IN, Pin.PULL_UP)
led = Pin(16, Pin.OUT)

while True:
    if button.value() == 0:      # 0 means the button is down
        led.on()
        time.sleep(3)
        
        led.value(0) # turn the  LED OFF
        time.sleep(2)
    else:
        led.off()
    time.sleep(1)