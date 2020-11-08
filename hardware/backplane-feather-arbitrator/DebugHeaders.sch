EESchema Schematic File Version 4
EELAYER 30 0
EELAYER END
$Descr A4 11693 8268
encoding utf-8
Sheet 2 2
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
L MiscellaneousDevices:ADAFRUIT_FEATHER U?
U 1 1 5F93816A
P 3650 5900
AR Path="/5F93816A" Ref="U?"  Part="1" 
AR Path="/5F9273C1/5F93816A" Ref="U2"  Part="1" 
AR Path="/5F8D76B4/5F93816A" Ref="U2"  Part="1" 
F 0 "U2" H 3828 6006 60  0000 L CNN
F 1 "ADAFRUIT_FEATHER" H 3828 5900 60  0000 L CNN
F 2 "Modules:ADAFRUIT_FEATHER_NOHOLES" H 3050 5200 60  0001 L CNN
F 3 "" H 3800 5700 60  0000 C CNN
	1    3650 5900
	1    0    0    -1  
$EndComp
$Comp
L MiscellaneousDevices:ADAFRUIT_FEATHER U?
U 2 1 5F938170
P 5400 3300
AR Path="/5F938170" Ref="U?"  Part="2" 
AR Path="/5F9273C1/5F938170" Ref="U3"  Part="2" 
AR Path="/5F8D76B4/5F938170" Ref="U3"  Part="2" 
F 0 "U3" H 5578 3406 60  0000 L CNN
F 1 "ADAFRUIT_FEATHER" H 5578 3300 60  0000 L CNN
F 2 "Modules:ADAFRUIT_FEATHER_NOHOLES" H 5578 3194 60  0000 L CNN
F 3 "" H 5550 3100 60  0000 C CNN
	2    5400 3300
	1    0    0    -1  
$EndComp
$Comp
L MiscellaneousDevices:ADAFRUIT_FEATHER U?
U 1 1 5F939A02
P 5300 5900
AR Path="/5F939A02" Ref="U?"  Part="1" 
AR Path="/5F9273C1/5F939A02" Ref="U3"  Part="1" 
AR Path="/5F8D76B4/5F939A02" Ref="U3"  Part="1" 
F 0 "U3" H 5478 6006 60  0000 L CNN
F 1 "ADAFRUIT_FEATHER" H 5478 5900 60  0000 L CNN
F 2 "Modules:ADAFRUIT_FEATHER_NOHOLES" H 4700 5200 60  0001 L CNN
F 3 "" H 5450 5700 60  0000 C CNN
	1    5300 5900
	1    0    0    -1  
$EndComp
$Comp
L MiscellaneousDevices:ADAFRUIT_FEATHER U?
U 2 1 5F939A08
P 6600 3300
AR Path="/5F939A08" Ref="U?"  Part="2" 
AR Path="/5F9273C1/5F939A08" Ref="U4"  Part="2" 
AR Path="/5F8D76B4/5F939A08" Ref="U4"  Part="2" 
F 0 "U4" H 6778 3406 60  0000 L CNN
F 1 "ADAFRUIT_FEATHER" H 6778 3300 60  0000 L CNN
F 2 "Modules:ADAFRUIT_FEATHER" H 6778 3194 60  0000 L CNN
F 3 "" H 6750 3100 60  0000 C CNN
	2    6600 3300
	1    0    0    -1  
$EndComp
Text HLabel 4450 2550 0    50   Input ~ 0
RST
Text HLabel 4450 2650 0    50   Input ~ 0
+3v3
Text HLabel 4450 2750 0    50   Input ~ 0
AREF
Text HLabel 4450 2850 0    50   Input ~ 0
GND
Text HLabel 4450 2950 0    50   Input ~ 0
A0
Text HLabel 4450 3050 0    50   Input ~ 0
A1
Text HLabel 4450 3150 0    50   Input ~ 0
A2
Text HLabel 4450 3250 0    50   Input ~ 0
A3
Text HLabel 4450 3350 0    50   Input ~ 0
A4
Text HLabel 4450 3450 0    50   Input ~ 0
A5
Text HLabel 4450 3550 0    50   Input ~ 0
SCK
Text HLabel 4450 3650 0    50   Input ~ 0
MOSI
Text HLabel 4450 3750 0    50   Input ~ 0
MISO
Text HLabel 4450 3850 0    50   Input ~ 0
D0
Text HLabel 4450 3950 0    50   Input ~ 0
D1
Text HLabel 4450 4050 0    50   Input ~ 0
DIO1
Text HLabel 3100 5350 0    50   Input ~ 0
VBAT
Text HLabel 3100 5450 0    50   Input ~ 0
EN
Text HLabel 3100 5550 0    50   Input ~ 0
VBUS
Text HLabel 3100 5650 0    50   Input ~ 0
D13
Text HLabel 3100 5750 0    50   Input ~ 0
D12
Text HLabel 3100 5850 0    50   Input ~ 0
D11
Text HLabel 3100 5950 0    50   Input ~ 0
D10
Text HLabel 3100 6050 0    50   Input ~ 0
D9
Text HLabel 3100 6150 0    50   Input ~ 0
D6
Text HLabel 3100 6250 0    50   Input ~ 0
D5
Text HLabel 3100 6350 0    50   Input ~ 0
D3
Text HLabel 3100 6450 0    50   Input ~ 0
D2
Wire Wire Line
	3100 5350 3400 5350
Connection ~ 3400 5350
Wire Wire Line
	3400 5950 3100 5950
Connection ~ 3400 5950
Wire Wire Line
	3100 6050 3400 6050
Connection ~ 3400 6050
Wire Wire Line
	3100 6150 3400 6150
Connection ~ 3400 6150
Wire Wire Line
	3400 6250 3100 6250
Connection ~ 3400 6250
Wire Wire Line
	3100 6350 3400 6350
Connection ~ 3400 6350
Wire Wire Line
	3400 6450 3100 6450
Connection ~ 3400 6450
Wire Wire Line
	3400 5850 3100 5850
Connection ~ 3400 5850
Wire Wire Line
	3400 5750 3100 5750
Connection ~ 3400 5750
Wire Wire Line
	3400 5650 3100 5650
Connection ~ 3400 5650
Wire Wire Line
	3400 5550 3100 5550
Connection ~ 3400 5550
Wire Wire Line
	3400 5450 3100 5450
Connection ~ 3400 5450
Wire Wire Line
	4450 2550 5150 2550
$Comp
L MiscellaneousDevices:ADAFRUIT_FEATHER U?
U 2 1 5F938A1E
P 6000 3300
AR Path="/5F938A1E" Ref="U?"  Part="1" 
AR Path="/5F9273C1/5F938A1E" Ref="U2"  Part="2" 
AR Path="/5F8D76B4/5F938A1E" Ref="U2"  Part="2" 
F 0 "U2" H 6178 3406 60  0000 L CNN
F 1 "ADAFRUIT_FEATHER" H 6178 3300 60  0000 L CNN
F 2 "Modules:ADAFRUIT_FEATHER_NOHOLES" H 5400 2600 60  0001 L CNN
F 3 "" H 6150 3100 60  0000 C CNN
	2    6000 3300
	1    0    0    -1  
$EndComp
Wire Wire Line
	4450 4050 5150 4050
Wire Wire Line
	4450 3950 5150 3950
Wire Wire Line
	4450 3850 5150 3850
Wire Wire Line
	4450 3750 5150 3750
Wire Wire Line
	5150 3650 4450 3650
Wire Wire Line
	4450 3550 5150 3550
Wire Wire Line
	5150 3450 4450 3450
Wire Wire Line
	4450 3350 5150 3350
Wire Wire Line
	5150 3250 4450 3250
Wire Wire Line
	4450 2950 5150 2950
Wire Wire Line
	5150 2850 4450 2850
Wire Wire Line
	4450 2750 5150 2750
$Comp
L MiscellaneousDevices:ADAFRUIT_FEATHER U?
U 1 1 5F938A24
P 4500 5900
AR Path="/5F938A24" Ref="U?"  Part="2" 
AR Path="/5F9273C1/5F938A24" Ref="U4"  Part="1" 
AR Path="/5F8D76B4/5F938A24" Ref="U4"  Part="1" 
F 0 "U4" H 4678 6006 60  0000 L CNN
F 1 "ADAFRUIT_FEATHER" H 4678 5900 60  0000 L CNN
F 2 "Modules:ADAFRUIT_FEATHER" H 4678 5794 60  0000 L CNN
F 3 "" H 4650 5700 60  0000 C CNN
	1    4500 5900
	1    0    0    -1  
$EndComp
Wire Wire Line
	3400 5350 4250 5350
Wire Wire Line
	4250 5350 5050 5350
Connection ~ 4250 5350
Wire Wire Line
	3400 5450 4250 5450
Wire Wire Line
	4250 5450 5050 5450
Connection ~ 4250 5450
Wire Wire Line
	3400 5550 4250 5550
Wire Wire Line
	4250 5550 5050 5550
Connection ~ 4250 5550
Wire Wire Line
	3400 5650 4250 5650
Wire Wire Line
	4250 5650 5050 5650
Connection ~ 4250 5650
Wire Wire Line
	3400 5750 4250 5750
Wire Wire Line
	4250 5750 5050 5750
Connection ~ 4250 5750
Wire Wire Line
	3400 5850 4250 5850
Wire Wire Line
	4250 5850 5050 5850
Connection ~ 4250 5850
Wire Wire Line
	3400 5950 4250 5950
Wire Wire Line
	4250 5950 5050 5950
Connection ~ 4250 5950
Wire Wire Line
	3400 6050 4250 6050
Wire Wire Line
	4250 6050 5050 6050
Connection ~ 4250 6050
Wire Wire Line
	3400 6150 4250 6150
Wire Wire Line
	4250 6150 5050 6150
Connection ~ 4250 6150
Wire Wire Line
	3400 6250 4250 6250
Wire Wire Line
	4250 6250 5050 6250
Connection ~ 4250 6250
Wire Wire Line
	3400 6350 4250 6350
Wire Wire Line
	4250 6350 5050 6350
Connection ~ 4250 6350
Wire Wire Line
	3400 6450 4250 6450
Wire Wire Line
	4250 6450 5050 6450
Connection ~ 4250 6450
Wire Wire Line
	4450 3150 5150 3150
Wire Wire Line
	5150 3050 4450 3050
Wire Wire Line
	5150 2550 5750 2550
Connection ~ 5150 2550
Wire Wire Line
	5150 2650 4450 2650
Wire Wire Line
	5150 2650 5750 2650
Connection ~ 5150 2650
Wire Wire Line
	5750 2650 6350 2650
Connection ~ 5750 2650
Wire Wire Line
	6350 2550 5750 2550
Connection ~ 5750 2550
Wire Wire Line
	5150 2750 5750 2750
Connection ~ 5150 2750
Wire Wire Line
	5750 2750 6350 2750
Connection ~ 5750 2750
Wire Wire Line
	6350 2850 5750 2850
Wire Wire Line
	5750 2850 5150 2850
Connection ~ 5750 2850
Connection ~ 5150 2850
Wire Wire Line
	5150 2950 5750 2950
Connection ~ 5150 2950
Wire Wire Line
	5750 2950 6350 2950
Connection ~ 5750 2950
Wire Wire Line
	5150 3050 5750 3050
Connection ~ 5150 3050
Wire Wire Line
	5750 3050 6350 3050
Connection ~ 5750 3050
Wire Wire Line
	5150 3150 5750 3150
Connection ~ 5150 3150
Wire Wire Line
	5750 3150 6350 3150
Connection ~ 5750 3150
Wire Wire Line
	6350 3250 5750 3250
Wire Wire Line
	5750 3250 5150 3250
Connection ~ 5750 3250
Connection ~ 5150 3250
Wire Wire Line
	5150 3350 5750 3350
Connection ~ 5150 3350
Wire Wire Line
	5750 3350 6350 3350
Connection ~ 5750 3350
Wire Wire Line
	6350 3450 5750 3450
Wire Wire Line
	5150 3450 5750 3450
Connection ~ 5150 3450
Connection ~ 5750 3450
Wire Wire Line
	5750 3550 5150 3550
Connection ~ 5150 3550
Wire Wire Line
	5750 3550 6350 3550
Connection ~ 5750 3550
Wire Wire Line
	6350 3650 5750 3650
Wire Wire Line
	5750 3650 5150 3650
Connection ~ 5750 3650
Connection ~ 5150 3650
Wire Wire Line
	5150 3750 5750 3750
Connection ~ 5150 3750
Wire Wire Line
	5750 3750 6350 3750
Connection ~ 5750 3750
Wire Wire Line
	6350 3850 5750 3850
Wire Wire Line
	5750 3850 5150 3850
Connection ~ 5750 3850
Connection ~ 5150 3850
Wire Wire Line
	5150 3950 5750 3950
Connection ~ 5150 3950
Wire Wire Line
	5750 3950 6350 3950
Connection ~ 5750 3950
Wire Wire Line
	6350 4050 5750 4050
Wire Wire Line
	5750 4050 5150 4050
Connection ~ 5750 4050
Connection ~ 5150 4050
$EndSCHEMATC
