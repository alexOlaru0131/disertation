# Feather M0 Express + ICM-20948

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
