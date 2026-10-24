# Feather M0 Express + ICM-20948 + PCA9685

Firmware-ul răspunde la comanda USB CDC **`0x10`** cu accelerația, viteza
unghiulară, câmpul magnetic și temperatura. Senzorul ICM-20948 este la adresa
I²C `0x69`; magnetometrul AK09916 este accesat la `0x0C` prin bypass-ul său.

## Flash de pe laptop

La prima încărcare, apasă **de două ori RESET** pe Feather ca să apară volumul
`FEATHERBOOT`, apoi rulează `cargo hf2 --release`. Proiectul este legat pentru
bootloader-ul Adafruit UF2 la adresa `0x2000`.

Pentru încărcările următoare, închide orice terminal care utilizează portul
serial și rulează `./build.sh`. Scriptul compilează, deschide portul aplicației
la 1200 bps și îl închide (intrare automată în bootloader), apoi rulează
`cargo hf2 --release` și cere prima măsurătoare. Sunt necesare `cargo-hf2`,
Python 3 și `pyserial`.

Pentru a intra doar în bootloader:

```sh
python3 enter_bootloader.py /dev/serial/by-id/usb-Alex_Software_Racheta_Debug_001-if00
```

## Formatul răspunsului

Trimite un singur octet `0x10`; firmware-ul efectuează o citire la cerere și
întoarce **41 de octeți**: `0x10`, urmat de zece numere `i32` semnate,
little-endian, fiecare egal cu valoarea fizică înmulțită cu 100. Ordinea este:

| Indici | Valori | Unitate |
| --- | --- | --- |
| 0–2 | accelerație X, Y, Z | m/s² |
| 3–5 | giroscop X, Y, Z | rad/s |
| 6–8 | magnetometru X, Y, Z | µT (submultiplu SI al tesla) |
| 9 | temperatură | °C |

Împarte fiecare număr la 100 pentru valoarea cu două zecimale. `i32` permite
și valori negative pe axe, fără limita de 655,35 a unui `u16`. Magnetometrul
folosește µT pentru a nu pierde întreg câmpul terestru prin rotunjire la
`T × 100`.

Dacă nu se pot obține toate valorile, răspunsul este un cadru de **4 octeți**:
`0xE0`, cod de eroare, detaliu, supliment. Coduri: 1 = eroare I²C
(detaliul este etapa, suplimentul este tipul erorii), 2 = ID ICM greșit,
3 = bank greșit, 4 = configurație accelerație greșită, 5 = ID magnetometru
greșit, 6 = magnetometru nepregătit, 7 = overflow magnetometru,
8 = configurație giroscop greșită.

Pentru codul 1, etapele din octetul `detaliu` sunt: 1 selectare bank,
2 citire registru, 3 trezire, 4 activare axe, 5 configurație accelerație,
6 configurație giroscop, 7 dezactivare master intern, 8 bypass magnetometru,
9 identificare magnetometru, 10 pornire magnetometru, 11 citire
accelerație/giroscop/temperatură, 12 citire magnetometru. Octetul `supliment`
indică 1 NACK, 2 eroare de magistrală, 3 pierdere arbitraj sau 4 overflow.

Exemplu pentru a cere și a decoda un cadru:

```sh
python3 read_imu.py
```

## PCA9685

Cele două module PCA9685 de la adresele I²C `0x41` și `0x42` folosesc aceeași
magistrală ca IMU-ul. La prima comandă PWM pentru fiecare modul, firmware-ul
citește MODE1 și MODE2, activează OUTDRV, intră în sleep pentru a seta
prescalerul `0x79` (~50 Hz), revine din sleep, așteaptă 5 ms și activează
RESTART și auto-increment. Fiecare modul comandă 10 canale cu impulsuri de
500–2400 µs pentru unghiuri între 0° și 360°.

După flash, pentru a comanda toate cele 20 de canale la 90° și a verifica
răspunsul plăcii:

```sh
python3 test_pca9685.py 9000
```

Comanda USB este `0x20` urmat de **20 de unghiuri `u16` little-endian** (40 de
octeți): primele 10 merg la `0x41`, următoarele 10 la `0x42`. Unghiurile sunt
în sutimi de grad, între `0` și `36000` (`9000` = 90°); toate cele 20 sunt
validate înainte de a fi trimise pe I²C. Scriptul acceptă fie un unghi pentru
toate canalele, fie 20 de unghiuri separate în această ordine. Răspunsul de
succes `[0x20, 20]` confirmă scrierea ambelor module. O eroare produce
`[0xE1, adresă, cod, etapă, detaliu]`, unde adresa este `0x41` sau `0x42`:
codul 1 indică o eroare I²C (etapele: 1 citire registru, 2 MODE2/OUTDRV,
3 sleep, 4 prescaler, 5 wake, 6 restart, 7 PWM; detaliul: 1 NACK,
2 eroare magistrală, 3 pierdere arbitraj, 4 overflow), codul 2 indică un
număr invalid de canale, codul 3 un număr insuficient de unghiuri, iar codul 4
un unghi peste 36000 (etapa este indicele canalului 0–9). Modulele sunt
inițializate și actualizate independent; dacă unul nu răspunde, celălalt
primește totuși comanda.

## LED pe A0

Conectează A0 la anodul LED-ului printr-o rezistență de 470 Ω, iar catodul
LED-ului la GND. LED-ul are nevoie de rezistența în serie când este alimentat
din pinul GPIO.

Pinul A0 este configurat ca ieșire digitală, inițial la nivel LOW (LED stins).
Trimite octetul `0x30` prin USB CDC pentru a-l aprinde (HIGH) sau `0x35`
pentru a-l stinge (LOW). Comenzile LED nu trimit răspuns USB.

Pentru a testa ambele stări, cu confirmare vizuală la fiecare pas:

```sh
python3 test_led.py
```

Scriptul stinge LED-ul și dacă testul este întrerupt. Poți folosi
`python3 test_led.py --no-confirm` pentru a trimite automat comenzile fără
confirmarea vizuală a stării LED-ului; în acest mod LED-ul rămâne aprins o
secundă. În testul interactiv, rămâne aprins până răspunzi la prima întrebare.
