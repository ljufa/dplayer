EESchema Schematic File Version 4
EELAYER 30 0
EELAYER END
$Descr A4 11693 8268
encoding utf-8
Sheet 1 8
Title ""
Date ""
Rev ""
Comp ""
Comment1 ""
Comment2 ""
Comment3 ""
Comment4 ""
$EndDescr
$Sheet
S 3200 700  1500 1550
U 600F3298
F0 "out_selector_and_mute" 50
F1 "out_selector.sch" 50
F2 "GPIO9" I L 3200 1050 50 
F3 "AUDIO_STREAMING" I L 3200 950 50 
F4 "GND_out" O L 3200 1450 50 
F5 "R_out" O L 3200 1650 50 
F6 "L_out" O L 3200 1550 50 
F7 "AGND_in" I R 4700 1750 50 
F8 "AGND_out_phn" O R 4700 1650 50 
F9 "AGND_out_spk" O R 4700 1550 50 
F10 "AR_out_pnh" O R 4700 1450 50 
F11 "AR_in" I R 4700 1350 50 
F12 "AR_out_spk" O R 4700 1250 50 
F13 "AL_out_phn" O R 4700 1150 50 
F14 "AL_in" I R 4700 1050 50 
F15 "AL_out_spk" O R 4700 950 50 
F16 "DAC_OUT_GND" I L 3200 1850 50 
F17 "DAC_OUT_R" I L 3200 2050 50 
F18 "DAC_OUT_L" I L 3200 1950 50 
F19 "5V_SWITCH" I L 3200 850 50 
$EndSheet
$Sheet
S 4900 2700 1200 850 
U 6011CE73
F0 "waveIO" 50
F1 "waveIO.sch" 50
F2 "AUDIO_STREAMING" O L 4900 2800 50 
F3 "I2S_DT" O L 4900 2900 50 
F4 "I2S_LR" O L 4900 3000 50 
F5 "I2S_BC" O L 4900 3100 50 
F6 "I2S_MC" O L 4900 3200 50 
F7 "I2S_V+" I L 4900 3300 50 
F8 "I2S_GND" B L 4900 3400 50 
F9 "USB_GND" I R 6100 2950 50 
F10 "USB_DT+" I R 6100 3050 50 
F11 "USB_DT-" I R 6100 3150 50 
F12 "USB_SHD" I R 6100 3250 50 
$EndSheet
$Sheet
S 4900 1500 1200 1000
U 60122709
F0 "dac_diyinhk_4497" 50
F1 "dac.sch" 50
F2 "I2S_MC" I R 6100 2400 50 
F3 "I2S_LR" I R 6100 2300 50 
F4 "I2S_BC" I R 6100 2200 50 
F5 "I2S_DT" I R 6100 2100 50 
F6 "I2S_V+" O R 6100 2000 50 
F7 "I2S_GND" I R 6100 1900 50 
F8 "GPIO3" I R 6100 1750 50 
F9 "GPIO2" I R 6100 1650 50 
F10 "GPIO14" I R 6100 1550 50 
F11 "DAC_OUT_R" O L 4900 1550 50 
F12 "DAC_OUT_GND" O L 4900 1750 50 
F13 "DAC_OUT_L" O L 4900 1650 50 
$EndSheet
$Sheet
S 4900 700  1200 600 
U 600F9AC5
F0 "lcd_st7920" 50
F1 "lcd.sch" 50
F2 "5V_SWITCH" I R 6100 800 50 
F3 "GPIO25" I R 6100 950 50 
F4 "GPIO10" I R 6100 1050 50 
F5 "GPIO11" I R 6100 1150 50 
F6 "GPIO9" U L 4900 900 50 
F7 "GPIO8" I R 6100 1250 50 
$EndSheet
$Sheet
S 4900 3750 1200 600 
U 601098FA
F0 "rpi4" 50
F1 "RPI4.sch" 50
F2 "GLOBAL_EN" I L 4900 3850 50 
F3 "5V_RPI" O L 4900 4000 50 
$EndSheet
$Sheet
S 1900 700  1000 1550
U 600F21EA
F0 "control_board" 50
F1 "control_board.sch" 50
F2 "GPIO15" I L 1900 1100 50 
F3 "GPIO18" I L 1900 1200 50 
F4 "GPIO27" I L 1900 1300 50 
F5 "GPIO17" I L 1900 1000 50 
F6 "GPIO23" I L 1900 1500 50 
F7 "5V_SWITCH" O R 2900 900 50 
F8 "GPIO9" B L 1900 1850 50 
F9 "220V_IN" I R 2900 1050 50 
F10 "220V_OUT" O R 2900 1150 50 
F11 "5V_RPI" I R 2900 800 50 
F12 "GPIO22" B L 1900 1950 50 
$EndSheet
Text Notes 600  7650 0    50   ~ 0
— Power on procedure —\n1. Press power on\n2. rpi on boot set GPIO23 to high on control board relay. (config.txt)\n3. relay provide power to LED pwr ind, mute board, LCD, HA, DAC, WaveIO\n4. dplay starts when everything is ON\n\n— Power off procedure —\n1. dplay is running and listen power off button push GPIO pin \n2. when button is pressed, dplay execute poweroff command on RPI OS
$Sheet
S 700  700  800  700 
U 60107AD5
F0 "power_buttons_board" 50
F1 "power_btn_brd.sch" 50
F2 "GPIO22" I R 1500 800 50 
F3 "GLOBAL_EN" I R 1500 900 50 
F4 "5V_SWITCH" O R 1500 1050 50 
$EndSheet
$EndSCHEMATC
