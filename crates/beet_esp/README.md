# Beet Esp

This crate is an exploration of the use of Bevy & Beet in microcontrollers. The initial focus is on the ESP32-S3-WROOM-1-N16R8, purchasable for < $10, ie [here](https://www.aliexpress.us/item/1005004617322170.html). 


## Datasheets
- esp32-s3
	- [datasheet](https://www.espressif.com/sites/default/files/documentation/esp32-s3-wroom-1_wroom-1u_datasheet_en.pdf)
	- [pinout](https://docs.espressif.com/projects/esp-idf/en/latest/esp32s3/_images/ESP32-S3_DevKitC-1_pinlayout.jpg)
- HC-SR04
	- https://www.sparkfun.com/products/15569
- 


### USB-IPD
```admin powershell
winget install usbipd
# update number as required
usbipd list
usbipd bind -b 7-1
usbipd attach -a -w -b 7-1
```

```sh
# check its working, esp32c3: /dev/ttyUSB0, esp32s3 COM port: /dev/ttyACM0
ls /dev/tty*

```

### `.env`
Create a `.env` file:
```
WIFI_SSID = "YOUR_SSID"
WIFI_PASS = "YOUR_PASS"
```
### espup-wsl

```sh
sudo apt-get install git wget flex bison gperf python3 python3-pip python3-venv cmake ninja-build ccache libffi-dev libssl-dev dfu-util libusb-1.0-0
cargo binstall --no-confirm espup ldproxy cargo-espflash espmonitor
espup install --targets esp32c3,esp32s3
code 
# ADD: . /home/pete/export-esp.sh
```
