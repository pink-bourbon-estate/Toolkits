from machine import Pin
import time

# led=Pin("LED", Pin.OUT) # Set up pin 16 which is connected to the LED
led=Pin(16, Pin.OUT) # Set up pin 16 which is connected to the LED

while True: # Loop forever
        led.value(1) # Turn the LED ON
        time.sleep(1)
        
        led.value(0) # turn the  LED OFF
        time.sleep(1)
        
        