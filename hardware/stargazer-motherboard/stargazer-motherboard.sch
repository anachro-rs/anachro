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
L MiscellaneousDevices:ADAFRUIT_FEATHER U1_n
U 2 1 5F83756C
P 2800 3850
F 0 "U1_n" H 2650 3000 60  0000 L CNN
F 1 "ADAFRUIT_FEATHER" H 2550 2900 60  0000 L CNN
F 2 "" H 2950 3650 60  0000 C CNN
F 3 "" H 2950 3650 60  0000 C CNN
	2    2800 3850
	1    0    0    -1  
$EndComp
$Comp
L MiscellaneousDevices:ADAFRUIT_FEATHER U1_n
U 1 1 5F836E05
P 3100 4000
F 0 "U1_n" H 2950 3300 60  0000 L CNN
F 1 "ADAFRUIT_FEATHER" H 2450 3200 60  0001 L CNN
F 2 "" H 3250 3800 60  0000 C CNN
F 3 "" H 3250 3800 60  0000 C CNN
	1    3100 4000
	-1   0    0    -1  
$EndComp
Text Label 2150 4100 2    50   ~ 0
CARDn-SCK
Text Label 2150 4200 2    50   ~ 0
CARDn-COPI
Text Label 2150 4300 2    50   ~ 0
CARDn-CIPO
Text Label 2150 4000 2    50   ~ 0
CARDn-CSn
Text Label 2150 3900 2    50   ~ 0
CARDn-GO
Wire Wire Line
	2150 3900 2550 3900
Wire Wire Line
	2150 4000 2550 4000
Wire Wire Line
	2150 4100 2550 4100
Wire Wire Line
	2150 4200 2550 4200
Wire Wire Line
	2150 4300 2550 4300
$Comp
L stargazer:feather-leftovers J3_n
U 1 1 5F93EF32
P 5800 950
F 0 "J3_n" H 5800 1100 50  0000 C CNN
F 1 "feather-leftovers" H 5800 1000 50  0000 C CNN
F 2 "" H 5800 950 50  0001 C CNN
F 3 "" H 5800 950 50  0001 C CNN
	1    5800 950 
	1    0    0    -1  
$EndComp
$Comp
L power:+3V3 #PWR?
U 1 1 5F871FB1
P 6100 6550
F 0 "#PWR?" H 6100 6400 50  0001 C CNN
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
L power:GND #PWR?
U 1 1 5F897D85
P 4950 7150
F 0 "#PWR?" H 4950 6900 50  0001 C CNN
F 1 "GND" H 4955 6977 50  0000 C CNN
F 2 "" H 4950 7150 50  0001 C CNN
F 3 "" H 4950 7150 50  0001 C CNN
	1    4950 7150
	1    0    0    -1  
$EndComp
$Comp
L power:GND #PWR?
U 1 1 5F8A0135
P 3950 1450
F 0 "#PWR?" H 3950 1200 50  0001 C CNN
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
L stargazer:swd-flywire J2_n
U 1 1 5F8B27A0
P 3000 5050
F 0 "J2_n" H 2722 4821 50  0000 R CNN
F 1 "swd-flywire" H 2722 4912 50  0000 R CNN
F 2 "" H 3000 5200 50  0001 C CNN
F 3 "" H 3000 5200 50  0001 C CNN
	1    3000 5050
	1    0    0    -1  
$EndComp
$Comp
L stargazer:swd-header J1_n
U 1 1 5F89F414
P 3900 950
F 0 "J1_n" H 4178 838 50  0000 L CNN
F 1 "swd-header" H 4178 747 50  0000 L CNN
F 2 "" H 3900 1050 50  0001 C CNN
F 3 "" H 3900 1050 50  0001 C CNN
	1    3900 950 
	1    0    0    -1  
$EndComp
$Comp
L stargazer:SN74LVC125AD U2_n
U 1 1 5F8CA4F1
P 5350 6500
F 0 "U2_n" H 5350 6715 50  0000 C CNN
F 1 "SN74LVC125AD" H 5350 6624 50  0000 C CNN
F 2 "" H 5350 6600 50  0001 C CNN
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
Text Label 1550 3200 2    50   ~ 0
3v3_n
$Comp
L power:GND #PWR?
U 1 1 5F9C38A2
P 1550 3400
F 0 "#PWR?" H 1550 3150 50  0001 C CNN
F 1 "GND" V 1555 3272 50  0000 R CNN
F 2 "" H 1550 3400 50  0001 C CNN
F 3 "" H 1550 3400 50  0001 C CNN
	1    1550 3400
	0    1    1    0   
$EndComp
Text Label 1550 3500 2    50   ~ 0
A0_n
Text Label 1550 3600 2    50   ~ 0
A1_n
Text Label 1550 3700 2    50   ~ 0
A2_n
Text Label 1550 3800 2    50   ~ 0
A3_n
Text Label 4300 4550 0    50   ~ 0
D2_n
Text Label 4300 4450 0    50   ~ 0
D3_n
Text Label 4300 4350 0    50   ~ 0
D5_n
Text Label 4300 4250 0    50   ~ 0
D6_n
Text Label 4300 4150 0    50   ~ 0
D9_n
Text Label 4300 4050 0    50   ~ 0
D10_n
Text Label 4300 3950 0    50   ~ 0
D11_n
Text Label 4300 3850 0    50   ~ 0
D12_n
Text Label 4300 3750 0    50   ~ 0
D13_n
Wire Wire Line
	1550 3400 2550 3400
Wire Wire Line
	2550 3500 1550 3500
Wire Wire Line
	1550 3600 2550 3600
Wire Wire Line
	1550 3700 2550 3700
Wire Wire Line
	1550 3800 2550 3800
Wire Wire Line
	3350 4550 4300 4550
Wire Wire Line
	3350 4450 4300 4450
Wire Wire Line
	3350 4350 4300 4350
Wire Wire Line
	3350 4250 4300 4250
Wire Wire Line
	3350 4150 4300 4150
Wire Wire Line
	3350 4050 4300 4050
Wire Wire Line
	3350 3950 4300 3950
Wire Wire Line
	3350 3850 4300 3850
Wire Wire Line
	3350 3750 4300 3750
Wire Wire Line
	1550 3200 2550 3200
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
Text Label 1550 4400 2    50   ~ 0
D0_n
Text Label 1550 4500 2    50   ~ 0
D1_n
Text Label 1550 4600 2    50   ~ 0
AUX_n
Wire Wire Line
	1550 4400 2550 4400
Wire Wire Line
	1550 4500 2550 4500
Wire Wire Line
	1550 4600 2550 4600
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
L power:GND #PWR?
U 1 1 5FA13A11
P 6500 1450
F 0 "#PWR?" H 6500 1200 50  0001 C CNN
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
Text Label 5750 3400 2    50   ~ 0
3v3_n
Text Notes 4650 6000 0    50   ~ 0
CARDn pins that go to the MCU\nCARDx pins that go to the common bus
Text Notes 7400 7500 0    50   ~ 0
Card Template n/m
$Comp
L power:GND #PWR?
U 1 1 5FA99FFC
P 10700 3900
F 0 "#PWR?" H 10700 3650 50  0001 C CNN
F 1 "GND" V 10705 3772 50  0000 R CNN
F 2 "" H 10700 3900 50  0001 C CNN
F 3 "" H 10700 3900 50  0001 C CNN
	1    10700 3900
	0    -1   -1   0   
$EndComp
$Comp
L power:GND #PWR?
U 1 1 5FA9C940
P 8600 5600
F 0 "#PWR?" H 8600 5350 50  0001 C CNN
F 1 "GND" V 8605 5472 50  0000 R CNN
F 2 "" H 8600 5600 50  0001 C CNN
F 3 "" H 8600 5600 50  0001 C CNN
	1    8600 5600
	0    1    1    0   
$EndComp
Text Label 1550 3100 2    50   ~ 0
RSTn
Wire Wire Line
	1550 3100 2550 3100
Text Label 4300 3450 0    50   ~ 0
VBATn
Text Label 4300 3550 0    50   ~ 0
ENn
Text Label 4300 3650 0    50   ~ 0
VBUSn
Wire Wire Line
	3350 3450 4300 3450
Wire Wire Line
	3350 3550 4300 3550
Wire Wire Line
	3350 3650 4300 3650
Text Label 5750 3200 2    50   ~ 0
RSTn
Text Label 5750 3600 2    50   ~ 0
VBATn
Text Label 5750 3800 2    50   ~ 0
ENn
Text Label 5750 4000 2    50   ~ 0
VBUSn
Text Label 6500 3200 0    50   ~ 0
RST
Text Label 6500 3600 0    50   ~ 0
VBAT
Text Label 6500 3800 0    50   ~ 0
EN
$Comp
L power:+3V3 #PWR?
U 1 1 5FAC606D
P 6500 3400
F 0 "#PWR?" H 6500 3250 50  0001 C CNN
F 1 "+3V3" V 6515 3528 50  0000 L CNN
F 2 "" H 6500 3400 50  0001 C CNN
F 3 "" H 6500 3400 50  0001 C CNN
	1    6500 3400
	0    1    1    0   
$EndComp
$Comp
L power:+5V #PWR?
U 1 1 5FAC6504
P 6500 4000
F 0 "#PWR?" H 6500 3850 50  0001 C CNN
F 1 "+5V" V 6515 4128 50  0000 L CNN
F 2 "" H 6500 4000 50  0001 C CNN
F 3 "" H 6500 4000 50  0001 C CNN
	1    6500 4000
	0    1    1    0   
$EndComp
$Comp
L Jumper:SolderJumper_2_Bridged JP1_n
U 1 1 5FAC6ABE
P 6100 3200
F 0 "JP1_n" H 6300 3250 50  0000 C CNN
F 1 "SolderJumper_2_Bridged" H 6100 3314 50  0001 C CNN
F 2 "" H 6100 3200 50  0001 C CNN
F 3 "~" H 6100 3200 50  0001 C CNN
	1    6100 3200
	1    0    0    -1  
$EndComp
$Comp
L Jumper:SolderJumper_2_Open JP2_n
U 1 1 5FAD2EB8
P 6100 3400
F 0 "JP2_n" H 6300 3450 50  0000 C CNN
F 1 "SolderJumper_2_Open" H 6100 3514 50  0001 C CNN
F 2 "" H 6100 3400 50  0001 C CNN
F 3 "~" H 6100 3400 50  0001 C CNN
	1    6100 3400
	1    0    0    -1  
$EndComp
$Comp
L Jumper:SolderJumper_2_Open JP3_n
U 1 1 5FAD3179
P 6100 3600
F 0 "JP3_n" H 6300 3650 50  0000 C CNN
F 1 "SolderJumper_2_Open" H 6100 3714 50  0001 C CNN
F 2 "" H 6100 3600 50  0001 C CNN
F 3 "~" H 6100 3600 50  0001 C CNN
	1    6100 3600
	1    0    0    -1  
$EndComp
$Comp
L Jumper:SolderJumper_2_Open JP4_n
U 1 1 5FAD3686
P 6100 3800
F 0 "JP4_n" H 6300 3850 50  0000 C CNN
F 1 "SolderJumper_2_Open" H 6100 3914 50  0001 C CNN
F 2 "" H 6100 3800 50  0001 C CNN
F 3 "~" H 6100 3800 50  0001 C CNN
	1    6100 3800
	1    0    0    -1  
$EndComp
$Comp
L Jumper:SolderJumper_2_Bridged JP5_n
U 1 1 5FAD3A22
P 6100 4000
F 0 "JP5_n" H 6300 4050 50  0000 C CNN
F 1 "SolderJumper_2_Bridged" H 6100 4114 50  0001 C CNN
F 2 "" H 6100 4000 50  0001 C CNN
F 3 "~" H 6100 4000 50  0001 C CNN
	1    6100 4000
	1    0    0    -1  
$EndComp
Wire Wire Line
	5750 3200 5950 3200
Wire Wire Line
	5750 3400 5950 3400
Wire Wire Line
	5750 3600 5950 3600
Wire Wire Line
	5750 3800 5950 3800
Wire Wire Line
	5750 4000 5950 4000
Wire Wire Line
	6250 4000 6500 4000
Wire Wire Line
	6250 3800 6500 3800
Wire Wire Line
	6250 3600 6500 3600
Wire Wire Line
	6250 3400 6500 3400
Wire Wire Line
	6250 3200 6500 3200
Text Label 8600 5500 2    50   ~ 0
VBUSn
Text Label 8600 5700 2    50   ~ 0
3v3_n
Text Label 10700 5300 0    50   ~ 0
RSTn
Text Label 10700 4000 0    50   ~ 0
3v3_n
Text Label 10700 3800 0    50   ~ 0
VBUSn
Wire Wire Line
	10250 3800 10700 3800
Wire Wire Line
	10250 3900 10700 3900
Wire Wire Line
	10250 4000 10700 4000
Wire Wire Line
	8600 5500 9150 5500
Wire Wire Line
	8600 5600 9150 5600
Wire Wire Line
	8600 5700 9150 5700
Text Label 9550 5950 3    50   ~ 0
3v3_n
$Comp
L power:GND #PWR?
U 1 1 5FB26C3E
P 9850 5950
F 0 "#PWR?" H 9850 5700 50  0001 C CNN
F 1 "GND" H 9855 5777 50  0000 C CNN
F 2 "" H 9850 5950 50  0001 C CNN
F 3 "" H 9850 5950 50  0001 C CNN
	1    9850 5950
	1    0    0    -1  
$EndComp
Wire Wire Line
	9550 5850 9550 5950
Wire Wire Line
	9850 5850 9850 5950
Wire Wire Line
	10250 5300 10700 5300
Text Label 8600 4800 2    50   ~ 0
CARDn-SCK
Text Label 8600 4900 2    50   ~ 0
CARDn-CIPO
Text Label 8600 5000 2    50   ~ 0
CARDn-COPI
Text Label 8600 4700 2    50   ~ 0
CARDn-CSn
Wire Wire Line
	8600 4700 9150 4700
Wire Wire Line
	8600 4800 9150 4800
Wire Wire Line
	8600 4900 9150 4900
Wire Wire Line
	8600 5000 9150 5000
Text Label 10700 5200 0    50   ~ 0
A0_n
Text Label 10700 5100 0    50   ~ 0
A1_n
Text Label 10700 5000 0    50   ~ 0
A2_n
Text Label 10700 4900 0    50   ~ 0
A3_n
Wire Wire Line
	10250 5200 10700 5200
Wire Wire Line
	10700 5100 10250 5100
Wire Wire Line
	10250 5000 10700 5000
Wire Wire Line
	10250 4900 10700 4900
Text Label 8600 5200 2    50   ~ 0
D0_n
Text Label 8600 5100 2    50   ~ 0
D1_n
Wire Wire Line
	8600 5100 9150 5100
Wire Wire Line
	8600 5200 9150 5200
Text Label 8600 5400 2    50   ~ 0
D2_n
Text Label 8600 5300 2    50   ~ 0
D3_n
Wire Wire Line
	8600 5300 9150 5300
Wire Wire Line
	8600 5400 9150 5400
Text Label 8600 4500 2    50   ~ 0
D5_n
Text Label 8600 4600 2    50   ~ 0
D6_n
Text Label 8600 3800 2    50   ~ 0
D9_n
Text Label 8600 3900 2    50   ~ 0
D10_n
Text Label 8600 4000 2    50   ~ 0
D11_n
Text Label 8600 4100 2    50   ~ 0
D12_n
Text Label 8600 4300 2    50   ~ 0
D13_n
Text Label 8600 4400 2    50   ~ 0
AUX_n
Wire Wire Line
	8600 4100 9150 4100
Wire Wire Line
	8600 4000 9150 4000
Wire Wire Line
	8600 3900 9150 3900
Wire Wire Line
	8600 3800 9150 3800
Wire Wire Line
	8600 4600 9150 4600
Wire Wire Line
	8600 4500 9150 4500
Wire Wire Line
	8600 4400 9150 4400
Wire Wire Line
	8600 4300 9150 4300
Text Label 10700 5700 0    50   ~ 0
VBATn
Wire Wire Line
	10250 5700 10700 5700
NoConn ~ 10250 5500
NoConn ~ 10250 5600
NoConn ~ 9150 4200
NoConn ~ 2550 3300
Text Notes 8650 5400 0    50   ~ 0
SDA1
Text Notes 8650 5300 0    50   ~ 0
SCL1
Text Notes 8650 5200 0    50   ~ 0
RX1/SDA1
Text Notes 8650 5100 0    50   ~ 0
TX1/SCL1
Text Notes 8650 4500 0    50   ~ 0
TX6
Text Notes 8650 4300 0    50   ~ 0
TX1
Text Notes 8650 4400 0    50   ~ 0
RX1
Text Notes 8650 4600 0    50   ~ 0
RX6
Text Notes 8650 4100 0    50   ~ 0
COPI2
Text Notes 8650 4000 0    50   ~ 0
CIPO2
Text Notes 8650 3900 0    50   ~ 0
SCK2
Text Notes 8650 3800 0    50   ~ 0
CSn2
Text Notes 10300 5200 0    50   ~ 0
ADC0
Text Notes 10300 5100 0    50   ~ 0
ADC1
Text Notes 10300 5000 0    50   ~ 0
ADC2
Text Notes 10300 4900 0    50   ~ 0
ADC3
$Comp
L stargazer:weact-black-pill U??
U 1 1 5FA97F9A
P 9700 3700
F 0 "U??" H 9700 3965 50  0000 C CNN
F 1 "weact-black-pill" H 9700 3874 50  0000 C CNN
F 2 "" H 9700 3850 50  0001 C CNN
F 3 "" H 9700 3850 50  0001 C CNN
	1    9700 3700
	1    0    0    -1  
$EndComp
Text Label 10700 5400 0    50   ~ 0
CARDn-GO
Wire Wire Line
	10250 5400 10700 5400
$Comp
L stargazer:black-pill-leftovers J??
U 1 1 5FC465DD
P 7700 1000
F 0 "J??" H 7650 1150 50  0000 L CNN
F 1 "black-pill-leftovers" H 7300 1050 50  0000 L CNN
F 2 "" H 7700 1150 50  0001 C CNN
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
Text Label 10700 4800 0    50   ~ 0
BP1_n
Text Label 10700 4700 0    50   ~ 0
BP2_n
Text Label 10700 4600 0    50   ~ 0
BP3_n
Text Label 10700 4500 0    50   ~ 0
BP4_n
Text Label 10700 4400 0    50   ~ 0
BP5_n
Text Label 10700 4300 0    50   ~ 0
BP6_n
Text Label 10700 4200 0    50   ~ 0
BP7_n
Text Label 10700 4100 0    50   ~ 0
BP8_n
Wire Wire Line
	10250 4100 10700 4100
Wire Wire Line
	10250 4200 10700 4200
Wire Wire Line
	10250 4300 10700 4300
Wire Wire Line
	10250 4400 10700 4400
Wire Wire Line
	10250 4500 10700 4500
Wire Wire Line
	10250 4600 10700 4600
Wire Wire Line
	10250 4700 10700 4700
Wire Wire Line
	10250 4800 10700 4800
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
Text Notes 10300 4600 0    50   ~ 0
CIPO1
Text Notes 10300 4500 0    50   ~ 0
COPI1
Text Notes 10300 4700 0    50   ~ 0
SCK1
Text Notes 10300 4800 0    50   ~ 0
CSn1
Text Notes 10300 4400 0    50   ~ 0
ADC8
Text Notes 10300 4300 0    50   ~ 0
ADC9
Text Notes 10300 4200 0    50   ~ 0
BOOT1
Wire Wire Line
	9750 5850 9750 6400
Text Label 9900 6400 0    50   ~ 0
SWCLKn
Text Label 9900 6500 0    50   ~ 0
SWDIOn
Wire Wire Line
	9750 6400 9900 6400
Wire Wire Line
	9650 6500 9900 6500
Wire Wire Line
	9650 5850 9650 6500
Text Label 3250 5600 0    50   ~ 0
SWCLKn
Text Label 3250 5700 0    50   ~ 0
SWDIOn
Wire Wire Line
	3100 5500 3100 5600
Wire Wire Line
	2900 5500 2900 5700
Wire Wire Line
	3100 5600 3250 5600
Wire Wire Line
	2900 5700 3250 5700
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
Text Notes 1650 4400 0    50   ~ 0
UART-RX
Text Notes 1650 4500 0    50   ~ 0
UART-TX
Text Notes 3450 4450 0    50   ~ 0
SCL
Text Notes 3450 4550 0    50   ~ 0
SDA
Text Notes 8650 5000 0    50   ~ 0
COPI1/3
Text Notes 8650 4900 0    50   ~ 0
CIPO1/3
Text Notes 8650 4800 0    50   ~ 0
SCK1/3
Text Notes 8650 4700 0    50   ~ 0
CSn1/3
$Comp
L stargazer:weact-black-pill U??
U 1 1 5FD4B16F
P 9700 1100
F 0 "U??" H 9700 1365 50  0000 C CNN
F 1 "weact-black-pill" H 9700 1274 50  0000 C CNN
F 2 "" H 9700 1250 50  0001 C CNN
F 3 "" H 9700 1250 50  0001 C CNN
	1    9700 1100
	1    0    0    -1  
$EndComp
Text Notes 8550 2300 0    50   ~ 0
SPI1_MISO
Text Notes 10450 1900 0    50   ~ 0
SPI1_MOSI
Text Notes 10450 2100 0    50   ~ 0
SPI1_SCK
Text Notes 10450 2200 0    50   ~ 0
SPI1_NSS
Text Notes 10450 2000 0    50   ~ 0
SDIO_CMD
Text Notes 8550 1500 0    50   ~ 0
SDIO_CLK
Text Notes 8550 1600 0    50   ~ 0
SDIO_D1
Text Notes 8550 1700 0    50   ~ 0
SDIO_D2
Text Notes 8550 2400 0    50   ~ 0
SDIO_D3
Text Notes 8550 2600 0    50   ~ 0
SDIO_D0
Wire Wire Line
	10250 1900 10850 1900
Wire Wire Line
	10250 2000 10850 2000
Wire Wire Line
	10250 2100 10850 2100
Wire Wire Line
	10250 2200 10850 2200
Wire Wire Line
	9150 2300 8550 2300
Wire Wire Line
	9150 2400 8550 2400
Wire Wire Line
	9150 2600 8550 2600
Wire Wire Line
	9150 1700 8550 1700
Wire Wire Line
	9150 1600 8550 1600
Wire Wire Line
	9150 1500 8550 1500
$EndSCHEMATC
