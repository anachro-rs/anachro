EESchema Schematic File Version 4
EELAYER 30 0
EELAYER END
$Descr A4 11693 8268
encoding utf-8
Sheet 1 1
Title ""
Date ""
Rev ""
Comp ""
Comment1 ""
Comment2 ""
Comment3 ""
Comment4 ""
$EndDescr
$Comp
L MiscellaneousDevices:ADAFRUIT_FEATHER U1_n1
U 1 1 5F83756C
P 3750 3800
F 0 "U1_n1" H 3500 3100 60  0000 L CNN
F 1 "ADAFRUIT_FEATHER" H 3500 3000 60  0000 L CNN
F 2 "Modules:ADAFRUIT_FEATHER" H 3900 3600 60  0001 C CNN
F 3 "" H 3900 3600 60  0000 C CNN
	1    3750 3800
	-1   0    0    -1  
$EndComp
$Comp
L MiscellaneousDevices:ADAFRUIT_FEATHER U1_n1
U 2 1 5F836E05
P 3450 3650
F 0 "U1_n1" H 3200 2800 60  0000 L CNN
F 1 "ADAFRUIT_FEATHER" H 2800 2850 60  0001 L CNN
F 2 "Modules:ADAFRUIT_FEATHER" H 3600 3450 60  0001 C CNN
F 3 "" H 3600 3450 60  0000 C CNN
	2    3450 3650
	1    0    0    -1  
$EndComp
Text Label 2800 3900 2    50   ~ 0
CARDn-SCK
Text Label 2800 4000 2    50   ~ 0
CARDn-COPI
Text Label 2800 4100 2    50   ~ 0
CARDn-CIPO
Text Label 2800 3800 2    50   ~ 0
CARDn-CSn
Text Label 2800 3700 2    50   ~ 0
CARDn-GO
Wire Wire Line
	2800 3700 3200 3700
Wire Wire Line
	2800 3800 3200 3800
Wire Wire Line
	2800 3900 3200 3900
Wire Wire Line
	2800 4000 3200 4000
Wire Wire Line
	2800 4100 3200 4100
$Comp
L stargazer:feather-leftovers J3_n1
U 1 1 5F93EF32
P 5800 950
F 0 "J3_n1" H 5800 1100 50  0000 C CNN
F 1 "feather-leftovers" H 5800 1000 50  0000 C CNN
F 2 "Connector_PinHeader_2.54mm:PinHeader_2x10_P2.54mm_Vertical" H 5800 950 50  0001 C CNN
F 3 "" H 5800 950 50  0001 C CNN
	1    5800 950 
	1    0    0    -1  
$EndComp
$Comp
L power:+3V3 #PWR0101
U 1 1 5F871FB1
P 6100 6550
F 0 "#PWR0101" H 6100 6400 50  0001 C CNN
F 1 "+3V3" H 6115 6723 50  0000 C CNN
F 2 "" H 6100 6550 50  0001 C CNN
F 3 "" H 6100 6550 50  0001 C CNN
	1    6100 6550
	1    0    0    -1  
$EndComp
Wire Wire Line
	5750 6550 6100 6550
Text Label 6000 6200 0    50   ~ 0
CARDn-GO
Text Label 4600 6750 2    50   ~ 0
CARDx-COPI
Text Label 4600 7050 2    50   ~ 0
CARDx-SCK
Text Label 6150 6850 0    50   ~ 0
CARDx-CSn
Text Label 6150 7050 0    50   ~ 0
CARDx-CIPO
Text Label 6150 6750 0    50   ~ 0
CARDn-CSn
Text Label 4600 6950 2    50   ~ 0
CARDn-SCK
Text Label 4600 6650 2    50   ~ 0
CARDn-COPI
Text Label 6150 7150 0    50   ~ 0
CARDn-CIPO
$Comp
L power:GND #PWR0102
U 1 1 5F897D85
P 4950 7150
F 0 "#PWR0102" H 4950 6900 50  0001 C CNN
F 1 "GND" H 4955 6977 50  0000 C CNN
F 2 "" H 4950 7150 50  0001 C CNN
F 3 "" H 4950 7150 50  0001 C CNN
	1    4950 7150
	1    0    0    -1  
$EndComp
$Comp
L power:GND #PWR0103
U 1 1 5F8A0135
P 3950 1450
F 0 "#PWR0103" H 3950 1200 50  0001 C CNN
F 1 "GND" H 3955 1277 50  0000 C CNN
F 2 "" H 3950 1450 50  0001 C CNN
F 3 "" H 3950 1450 50  0001 C CNN
	1    3950 1450
	1    0    0    -1  
$EndComp
Text Label 4150 1400 0    50   ~ 0
RSTn
Wire Wire Line
	4050 1300 4050 1400
Wire Wire Line
	4050 1400 4150 1400
Wire Wire Line
	3950 1300 3950 1450
$Comp
L stargazer:swd-flywire J2_n1
U 1 1 5F8B27A0
P 3600 4800
F 0 "J2_n1" H 3322 4571 50  0000 R CNN
F 1 "swd-flywire" H 3322 4662 50  0000 R CNN
F 2 "Connector_PinHeader_2.54mm:PinHeader_1x02_P2.54mm_Vertical" H 3600 4950 50  0001 C CNN
F 3 "" H 3600 4950 50  0001 C CNN
	1    3600 4800
	1    0    0    -1  
$EndComp
$Comp
L stargazer:swd-header J1_n1
U 1 1 5F89F414
P 3900 950
F 0 "J1_n1" H 4178 838 50  0000 L CNN
F 1 "swd-header" H 4178 747 50  0000 L CNN
F 2 "Connector_PinHeader_2.54mm:PinHeader_2x02_P2.54mm_Vertical" H 3900 1050 50  0001 C CNN
F 3 "" H 3900 1050 50  0001 C CNN
	1    3900 950 
	1    0    0    -1  
$EndComp
$Comp
L stargazer:SN74LVC125AD U2_n1
U 1 1 5F8CA4F1
P 5350 6500
F 0 "U2_n1" H 5350 6715 50  0000 C CNN
F 1 "SN74LVC125AD" H 5350 6624 50  0000 C CNN
F 2 "Package_SO:SOIC-14_3.9x8.7mm_P1.27mm" H 5350 6600 50  0001 C CNN
F 3 "" H 5350 6600 50  0001 C CNN
	1    5350 6500
	1    0    0    -1  
$EndComp
Wire Wire Line
	4950 6550 4850 6550
Wire Wire Line
	4700 6550 4700 6850
Wire Wire Line
	4700 6850 4950 6850
Wire Wire Line
	5750 6650 5850 6650
Wire Wire Line
	5950 6650 5950 6950
Wire Wire Line
	5950 6950 5750 6950
Wire Wire Line
	4850 6550 4850 6200
Wire Wire Line
	4850 6200 5850 6200
Wire Wire Line
	5850 6200 5850 6650
Connection ~ 4850 6550
Wire Wire Line
	4850 6550 4700 6550
Connection ~ 5850 6650
Wire Wire Line
	5850 6650 5950 6650
Wire Wire Line
	5850 6200 6000 6200
Connection ~ 5850 6200
Wire Wire Line
	4600 6650 4950 6650
Wire Wire Line
	4600 6950 4950 6950
Wire Wire Line
	5750 6750 6150 6750
Wire Wire Line
	5750 7050 6150 7050
Wire Wire Line
	4600 6750 4950 6750
Wire Wire Line
	4600 7050 4950 7050
Wire Wire Line
	5750 7150 6150 7150
Wire Wire Line
	5750 6850 6150 6850
Text Label 2200 3000 2    50   ~ 0
3v3_n
$Comp
L power:GND #PWR0104
U 1 1 5F9C38A2
P 2200 3200
F 0 "#PWR0104" H 2200 2950 50  0001 C CNN
F 1 "GND" V 2205 3072 50  0000 R CNN
F 2 "" H 2200 3200 50  0001 C CNN
F 3 "" H 2200 3200 50  0001 C CNN
	1    2200 3200
	0    1    1    0   
$EndComp
Text Label 2200 3300 2    50   ~ 0
A0_n
Text Label 2200 3400 2    50   ~ 0
A1_n
Text Label 2200 3500 2    50   ~ 0
A2_n
Text Label 2200 3600 2    50   ~ 0
A3_n
Text Label 4950 4350 0    50   ~ 0
D2_n
Text Label 4950 4250 0    50   ~ 0
D3_n
Text Label 4950 4150 0    50   ~ 0
D5_n
Text Label 4950 4050 0    50   ~ 0
D6_n
Text Label 4950 3950 0    50   ~ 0
D9_n
Text Label 4950 3850 0    50   ~ 0
D10_n
Text Label 4950 3750 0    50   ~ 0
D11_n
Text Label 4950 3650 0    50   ~ 0
D12_n
Text Label 4950 3550 0    50   ~ 0
D13_n
Wire Wire Line
	2200 3200 3200 3200
Wire Wire Line
	3200 3300 2200 3300
Wire Wire Line
	2200 3400 3200 3400
Wire Wire Line
	2200 3500 3200 3500
Wire Wire Line
	2200 3600 3200 3600
Wire Wire Line
	4000 4350 4950 4350
Wire Wire Line
	4000 4250 4950 4250
Wire Wire Line
	4000 4150 4950 4150
Wire Wire Line
	4000 4050 4950 4050
Wire Wire Line
	4000 3950 4950 3950
Wire Wire Line
	4000 3850 4950 3850
Wire Wire Line
	4000 3750 4950 3750
Wire Wire Line
	4000 3650 4950 3650
Wire Wire Line
	4000 3550 4950 3550
Wire Wire Line
	2200 3000 3200 3000
Text Label 5550 1450 3    50   ~ 0
D2_n
Text Label 5650 1450 3    50   ~ 0
D3_n
Text Label 5750 1450 3    50   ~ 0
D5_n
Text Label 5850 1450 3    50   ~ 0
D6_n
Text Label 5950 1450 3    50   ~ 0
D9_n
Text Label 6050 1450 3    50   ~ 0
D10_n
Text Label 6150 1450 3    50   ~ 0
D11_n
Text Label 6250 1450 3    50   ~ 0
D12_n
Text Label 6350 1450 3    50   ~ 0
D13_n
Text Label 2200 4200 2    50   ~ 0
D0_n
Text Label 2200 4300 2    50   ~ 0
D1_n
Text Label 2200 4400 2    50   ~ 0
AUX_n
Wire Wire Line
	2200 4200 3200 4200
Wire Wire Line
	2200 4300 3200 4300
Wire Wire Line
	2200 4400 3200 4400
Text Label 5250 1450 3    50   ~ 0
D0_n
Text Label 5350 1450 3    50   ~ 0
D1_n
Text Label 5450 1450 3    50   ~ 0
AUX_n
Text Label 4950 1450 3    50   ~ 0
A1_n
Text Label 5050 1450 3    50   ~ 0
A2_n
Text Label 5150 1450 3    50   ~ 0
A3_n
Text Label 4850 1450 3    50   ~ 0
A0_n
$Comp
L power:GND #PWR0105
U 1 1 5FA13A11
P 6500 1450
F 0 "#PWR0105" H 6500 1200 50  0001 C CNN
F 1 "GND" H 6505 1277 50  0000 C CNN
F 2 "" H 6500 1450 50  0001 C CNN
F 3 "" H 6500 1450 50  0001 C CNN
	1    6500 1450
	1    0    0    -1  
$EndComp
Text Label 6700 1450 0    50   ~ 0
3v3_n
Wire Wire Line
	6650 1300 6700 1300
Wire Wire Line
	6700 1300 6700 1450
Connection ~ 6700 1300
Wire Wire Line
	6700 1300 6750 1300
Wire Wire Line
	6450 1300 6500 1300
Wire Wire Line
	6500 1300 6500 1450
Connection ~ 6500 1300
Wire Wire Line
	6500 1300 6550 1300
Wire Wire Line
	6350 1450 6350 1300
Wire Wire Line
	6250 1300 6250 1450
Wire Wire Line
	6150 1300 6150 1450
Wire Wire Line
	6050 1300 6050 1450
Wire Wire Line
	5950 1300 5950 1450
Wire Wire Line
	5850 1300 5850 1450
Wire Wire Line
	5750 1300 5750 1450
Wire Wire Line
	5650 1300 5650 1450
Wire Wire Line
	5550 1300 5550 1450
Wire Wire Line
	5450 1300 5450 1450
Wire Wire Line
	5350 1300 5350 1450
Wire Wire Line
	5250 1300 5250 1450
Wire Wire Line
	5150 1300 5150 1450
Wire Wire Line
	5050 1300 5050 1450
Wire Wire Line
	4950 1300 4950 1450
Wire Wire Line
	4850 1300 4850 1450
Text Label 1000 1100 2    50   ~ 0
3v3_n
Text Notes 4650 6000 0    50   ~ 0
CARDn pins that go to the MCU\nCARDx pins that go to the common bus
Text Notes 7400 7500 0    50   ~ 0
Card Template n/m
Text Label 2200 2900 2    50   ~ 0
RSTn
Wire Wire Line
	2200 2900 3200 2900
Text Label 4950 3250 0    50   ~ 0
VBATn
Text Label 4950 3350 0    50   ~ 0
ENn
Text Label 4950 3450 0    50   ~ 0
VBUSn
Wire Wire Line
	4000 3250 4950 3250
Wire Wire Line
	4000 3350 4950 3350
Wire Wire Line
	4000 3450 4950 3450
Text Label 1000 900  2    50   ~ 0
RSTn
Text Label 1000 1300 2    50   ~ 0
VBATn
Text Label 1000 1500 2    50   ~ 0
ENn
Text Label 1000 1700 2    50   ~ 0
VBUSn
Text Label 1750 900  0    50   ~ 0
RST
Text Label 1750 1300 0    50   ~ 0
VBAT
Text Label 1750 1500 0    50   ~ 0
EN
$Comp
L power:+3V3 #PWR0108
U 1 1 5FAC606D
P 1750 1100
F 0 "#PWR0108" H 1750 950 50  0001 C CNN
F 1 "+3V3" V 1765 1228 50  0000 L CNN
F 2 "" H 1750 1100 50  0001 C CNN
F 3 "" H 1750 1100 50  0001 C CNN
	1    1750 1100
	0    1    1    0   
$EndComp
$Comp
L power:+5V #PWR0109
U 1 1 5FAC6504
P 1750 1700
F 0 "#PWR0109" H 1750 1550 50  0001 C CNN
F 1 "+5V" V 1765 1828 50  0000 L CNN
F 2 "" H 1750 1700 50  0001 C CNN
F 3 "" H 1750 1700 50  0001 C CNN
	1    1750 1700
	0    1    1    0   
$EndComp
$Comp
L Jumper:SolderJumper_2_Bridged JP1_n1
U 1 1 5FAC6ABE
P 1350 900
F 0 "JP1_n1" H 1550 950 50  0000 C CNN
F 1 "SolderJumper_2_Bridged" H 1350 1014 50  0001 C CNN
F 2 "Jumper:SolderJumper-2_P1.3mm_Bridged_RoundedPad1.0x1.5mm" H 1350 900 50  0001 C CNN
F 3 "~" H 1350 900 50  0001 C CNN
	1    1350 900 
	1    0    0    -1  
$EndComp
$Comp
L Jumper:SolderJumper_2_Open JP2_n1
U 1 1 5FAD2EB8
P 1350 1100
F 0 "JP2_n1" H 1550 1150 50  0000 C CNN
F 1 "SolderJumper_2_Open" H 1350 1214 50  0001 C CNN
F 2 "Jumper:SolderJumper-2_P1.3mm_Open_RoundedPad1.0x1.5mm" H 1350 1100 50  0001 C CNN
F 3 "~" H 1350 1100 50  0001 C CNN
	1    1350 1100
	1    0    0    -1  
$EndComp
$Comp
L Jumper:SolderJumper_2_Open JP3_n1
U 1 1 5FAD3179
P 1350 1300
F 0 "JP3_n1" H 1550 1350 50  0000 C CNN
F 1 "SolderJumper_2_Open" H 1350 1414 50  0001 C CNN
F 2 "Jumper:SolderJumper-2_P1.3mm_Open_RoundedPad1.0x1.5mm" H 1350 1300 50  0001 C CNN
F 3 "~" H 1350 1300 50  0001 C CNN
	1    1350 1300
	1    0    0    -1  
$EndComp
$Comp
L Jumper:SolderJumper_2_Open JP4_n1
U 1 1 5FAD3686
P 1350 1500
F 0 "JP4_n1" H 1550 1550 50  0000 C CNN
F 1 "SolderJumper_2_Open" H 1350 1614 50  0001 C CNN
F 2 "Jumper:SolderJumper-2_P1.3mm_Open_RoundedPad1.0x1.5mm" H 1350 1500 50  0001 C CNN
F 3 "~" H 1350 1500 50  0001 C CNN
	1    1350 1500
	1    0    0    -1  
$EndComp
$Comp
L Jumper:SolderJumper_2_Bridged JP5_n1
U 1 1 5FAD3A22
P 1350 1700
F 0 "JP5_n1" H 1550 1750 50  0000 C CNN
F 1 "SolderJumper_2_Bridged" H 1350 1814 50  0001 C CNN
F 2 "Jumper:SolderJumper-2_P1.3mm_Bridged_RoundedPad1.0x1.5mm" H 1350 1700 50  0001 C CNN
F 3 "~" H 1350 1700 50  0001 C CNN
	1    1350 1700
	1    0    0    -1  
$EndComp
Wire Wire Line
	1000 900  1200 900 
Wire Wire Line
	1000 1100 1200 1100
Wire Wire Line
	1000 1300 1200 1300
Wire Wire Line
	1000 1500 1200 1500
Wire Wire Line
	1000 1700 1200 1700
Wire Wire Line
	1500 1700 1750 1700
Wire Wire Line
	1500 1500 1750 1500
Wire Wire Line
	1500 1300 1750 1300
Wire Wire Line
	1500 1100 1750 1100
Wire Wire Line
	1500 900  1750 900 
$Comp
L stargazer:black-pill-leftovers J?1
U 1 1 5FC465DD
P 7700 1000
F 0 "J?1" H 7650 1150 50  0000 L CNN
F 1 "black-pill-leftovers" H 7300 1050 50  0000 L CNN
F 2 "Connector_PinHeader_2.54mm:PinHeader_2x04_P2.54mm_Vertical" H 7700 1150 50  0001 C CNN
F 3 "" H 7700 1150 50  0001 C CNN
	1    7700 1000
	1    0    0    -1  
$EndComp
Text Label 7350 1450 3    50   ~ 0
BP1_n
Text Label 7450 1450 3    50   ~ 0
BP2_n
Text Label 7550 1450 3    50   ~ 0
BP3_n
Text Label 7650 1450 3    50   ~ 0
BP4_n
Text Label 7750 1450 3    50   ~ 0
BP5_n
Text Label 7850 1450 3    50   ~ 0
BP6_n
Text Label 7950 1450 3    50   ~ 0
BP7_n
Text Label 8050 1450 3    50   ~ 0
BP8_n
Wire Wire Line
	7350 1450 7350 1300
Wire Wire Line
	7450 1450 7450 1300
Wire Wire Line
	7550 1450 7550 1300
Wire Wire Line
	7650 1450 7650 1300
Wire Wire Line
	7750 1450 7750 1300
Wire Wire Line
	7850 1450 7850 1300
Wire Wire Line
	7950 1450 7950 1300
Wire Wire Line
	8050 1450 8050 1300
Text Label 3850 5350 0    50   ~ 0
SWCLKn
Text Label 3850 5450 0    50   ~ 0
SWDIOn
Wire Wire Line
	3700 5250 3700 5350
Wire Wire Line
	3500 5250 3500 5450
Wire Wire Line
	3700 5350 3850 5350
Wire Wire Line
	3500 5450 3850 5450
Text Label 3550 1550 2    50   ~ 0
SWDIOn
Text Label 3550 1450 2    50   ~ 0
SWCLKn
Wire Wire Line
	3550 1550 3850 1550
Wire Wire Line
	3850 1550 3850 1300
Wire Wire Line
	3750 1300 3750 1450
Wire Wire Line
	3750 1450 3550 1450
Text Notes 2300 4200 0    50   ~ 0
UART-RX
Text Notes 2300 4300 0    50   ~ 0
UART-TX
Text Notes 4200 4250 2    50   ~ 0
SCL
Text Notes 4200 4350 2    50   ~ 0
SDA
$Comp
L stargazer:weact-black-pill U?1
U 1 1 5FD4B16F
P 8300 2950
F 0 "U?1" H 8300 3215 50  0000 C CNN
F 1 "weact-black-pill" H 8300 3124 50  0000 C CNN
F 2 "stargazer-motherboard:weact-blackpill" H 8300 3100 50  0001 C CNN
F 3 "" H 8300 3100 50  0001 C CNN
	1    8300 2950
	1    0    0    -1  
$EndComp
Text Notes 7150 4150 0    50   ~ 0
SPI1_MISO
Text Notes 8900 3750 0    50   ~ 0
SPI1_MOSI
Text Notes 8900 3950 0    50   ~ 0
SPI1_SCK
Text Notes 8900 4050 0    50   ~ 0
SPI1_NSS
Text Notes 8900 3850 0    50   ~ 0
SDIO_CMD
Text Notes 7150 3350 0    50   ~ 0
SDIO_CLK
Text Notes 7150 3450 0    50   ~ 0
SDIO_D1
Text Notes 7150 3550 0    50   ~ 0
SDIO_D2
Text Notes 7150 4250 0    50   ~ 0
SDIO_D3
Text Notes 7150 4450 0    50   ~ 0
SDIO_D0
Wire Wire Line
	7750 4150 7150 4150
Wire Wire Line
	7750 4250 7150 4250
Wire Wire Line
	7750 4450 7150 4450
Wire Wire Line
	7750 3550 7150 3550
Wire Wire Line
	7750 3450 7150 3450
Wire Wire Line
	7750 3350 7150 3350
Text Label 7150 4150 2    50   ~ 0
CARDn-CIPO
Text Label 9300 3750 0    50   ~ 0
CARDn-COPI
Text Label 9300 3950 0    50   ~ 0
CARDn-SCK
Text Label 9300 4050 0    50   ~ 0
CARDn-CSn
Text Label 7150 3350 2    50   ~ 0
BP1_n
Text Label 7150 4450 2    50   ~ 0
BP2_n
Text Label 7150 3450 2    50   ~ 0
BP3_n
Text Label 7150 3550 2    50   ~ 0
BP4_n
Text Label 7150 4250 2    50   ~ 0
BP5_n
Text Label 9300 3850 0    50   ~ 0
BP6_n
Text Label 9300 4450 0    50   ~ 0
A0_n
Text Label 9300 4350 0    50   ~ 0
A1_n
Text Label 9300 4250 0    50   ~ 0
A2_n
Text Label 9300 4150 0    50   ~ 0
A3_n
Wire Wire Line
	8850 4450 9300 4450
Wire Wire Line
	9300 4350 8850 4350
Wire Wire Line
	8850 4250 9300 4250
Wire Wire Line
	8850 4150 9300 4150
Text Notes 8900 4450 0    50   ~ 0
ADC0
Text Notes 8900 4350 0    50   ~ 0
ADC1
Text Notes 8900 4250 0    50   ~ 0
ADC2
Text Notes 8900 4150 0    50   ~ 0
ADC3
Text Notes 7150 4050 0    50   ~ 0
SDA2
Text Notes 9050 3350 0    50   ~ 0
SCL2
Text Label 7150 4050 2    50   ~ 0
D2_n
Text Label 9300 3350 0    50   ~ 0
D3_n
Wire Wire Line
	7150 4050 7750 4050
Text Notes 7150 3650 0    50   ~ 0
RX1
Text Notes 7150 3950 0    50   ~ 0
TX1
Text Label 7150 3650 2    50   ~ 0
D0_n
Text Label 7150 3950 2    50   ~ 0
D1_n
Wire Wire Line
	7150 3950 7750 3950
Wire Wire Line
	7150 3650 7750 3650
Text Label 7150 3750 2    50   ~ 0
BP7_n
Text Label 7150 3850 2    50   ~ 0
BP8_n
Wire Wire Line
	7150 3750 7750 3750
Wire Wire Line
	7750 3850 7150 3850
Text Notes 7150 3850 0    50   ~ 0
USB-DP
Text Notes 7150 3750 0    50   ~ 0
USB-DM
Text Label 7150 4550 2    50   ~ 0
D5_n
Text Label 7150 4650 2    50   ~ 0
D6_n
Wire Wire Line
	7150 4550 7750 4550
Wire Wire Line
	7150 4650 7750 4650
Text Notes 7150 4550 0    50   ~ 0
SCL1\n
Text Notes 7150 4650 0    50   ~ 0
SDA1
$Comp
L power:GND #PWR0106
U 1 1 5F8E7355
P 7200 4850
F 0 "#PWR0106" H 7200 4600 50  0001 C CNN
F 1 "GND" V 7205 4722 50  0000 R CNN
F 2 "" H 7200 4850 50  0001 C CNN
F 3 "" H 7200 4850 50  0001 C CNN
	1    7200 4850
	0    1    1    0   
$EndComp
Text Label 7200 4750 2    50   ~ 0
VBUSn
Text Label 7200 4950 2    50   ~ 0
3v3_n
Wire Wire Line
	7200 4750 7750 4750
Wire Wire Line
	7200 4850 7750 4850
Wire Wire Line
	7200 4950 7750 4950
Text Label 9300 4950 0    50   ~ 0
VBATn
Wire Wire Line
	8850 4950 9300 4950
Text Label 7150 4350 2    50   ~ 0
CARDn-GO
Wire Wire Line
	7150 4350 7750 4350
Text Label 8150 5200 3    50   ~ 0
3v3_n
$Comp
L power:GND #PWR0107
U 1 1 5F9048D3
P 8450 5200
F 0 "#PWR0107" H 8450 4950 50  0001 C CNN
F 1 "GND" H 8455 5027 50  0000 C CNN
F 2 "" H 8450 5200 50  0001 C CNN
F 3 "" H 8450 5200 50  0001 C CNN
	1    8450 5200
	1    0    0    -1  
$EndComp
Wire Wire Line
	8150 5100 8150 5200
Wire Wire Line
	8450 5100 8450 5200
Wire Wire Line
	8350 5100 8350 5650
Text Label 8500 5650 0    50   ~ 0
SWCLKn
Text Label 8500 5750 0    50   ~ 0
SWDIOn
Wire Wire Line
	8350 5650 8500 5650
Wire Wire Line
	8250 5750 8500 5750
Wire Wire Line
	8250 5100 8250 5750
$Comp
L power:GND #PWR0110
U 1 1 5F913B9B
P 9300 3150
F 0 "#PWR0110" H 9300 2900 50  0001 C CNN
F 1 "GND" V 9305 3022 50  0000 R CNN
F 2 "" H 9300 3150 50  0001 C CNN
F 3 "" H 9300 3150 50  0001 C CNN
	1    9300 3150
	0    -1   -1   0   
$EndComp
Text Label 9300 3250 0    50   ~ 0
3v3_n
Text Label 9300 3050 0    50   ~ 0
VBUSn
Wire Wire Line
	8850 3050 9300 3050
Wire Wire Line
	8850 3150 9300 3150
Wire Wire Line
	8850 3250 9300 3250
Text Label 9300 3650 0    50   ~ 0
D9_n
Text Label 9300 3550 0    50   ~ 0
D10_n
Text Label 9300 3450 0    50   ~ 0
D11_n
Text Label 7150 3050 2    50   ~ 0
D12_n
Text Label 7150 3150 2    50   ~ 0
D13_n
Text Label 7150 3250 2    50   ~ 0
AUX_n
Wire Wire Line
	7150 3050 7750 3050
Wire Wire Line
	7150 3150 7750 3150
Wire Wire Line
	7150 3250 7750 3250
Wire Wire Line
	8850 3650 9300 3650
Wire Wire Line
	9300 3550 8850 3550
Wire Wire Line
	8850 3450 9300 3450
Text Label 9300 4550 0    50   ~ 0
RSTn
Wire Wire Line
	8850 4550 9300 4550
NoConn ~ 8850 4650
NoConn ~ 8850 4750
NoConn ~ 8850 4850
Wire Wire Line
	8850 3350 9300 3350
Wire Wire Line
	8850 3750 9300 3750
Wire Wire Line
	9300 3850 8850 3850
Wire Wire Line
	8850 3950 9300 3950
Wire Wire Line
	8850 4050 9300 4050
NoConn ~ 3200 3100
Text Notes 5000 1900 0    50   ~ 0
TODO: I PROBABLY NEED TO REORGANIZE\nOR SPLIT UP THIS HEADER. ITS A PAIN TO ROUTE
$EndSCHEMATC
