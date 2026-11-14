# Pico 2 learning

**This is a learning project, not a tool.** It is me working out how a microcontroller does anything at all, with the code kept simple and readable on purpose. It belongs with the [Tutorials](https://github.com/pink-bourbon-estate/Tutorials) idea: understand it first, then own it.

Board: Raspberry Pi Pico 2 WH, running MicroPython.

| File | What it does |
| --- | --- |
| `light on light off.py` | Blinks an LED on pin 16: one second on, one second off, forever |
| `press the button to blink the light.py` | A button on pin 17 (pulled up, so pressed reads 0) lights the LED for three seconds, then off for two |

Wiring, as the code expects it: an LED (with a resistor) on GP16, a push button between GP17 and ground. I have not written down a tested circuit diagram yet, so check your own wiring before trusting it.
