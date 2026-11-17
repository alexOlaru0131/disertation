"""Meniu de test pentru Zero 2 W configurat ca USB serial gadget."""

import argparse
import json
import sys
from decimal import Decimal, InvalidOperation

import serial
from serial.tools import list_ports


def choose_port():
    ports = list(list_ports.comports())
    if ports:
        print("Porturi seriale disponibile:")
        for port in ports:
            print(f"  {port.device} — {port.description}")
    return input("Portul USB al Raspberry Pi (ex. /dev/ttyACM0): ").strip()


def angle_in_centidegrees(text):
    try:
        angle = Decimal(text.replace(",", "."))
    except InvalidOperation as error:
        raise ValueError("Introdu un unghi numeric în grade.") from error
    centidegrees = angle * 100
    if not angle.is_finite() or not 0 <= angle <= 360 or centidegrees != int(centidegrees):
        raise ValueError("Unghiul trebuie să fie între 0 și 360°, cu maximum două zecimale.")
    return int(centidegrees)


def ask_angles(count):
    while True:
        text = input(
            "Unghi în grade (ex. 90): " if count == 1 else
            "20 unghiuri în grade, separate prin spații: "
        )
        values = text.split()
        if len(values) != count:
            print(f"Sunt necesare exact {count} valori.")
            continue
        try:
            return [angle_in_centidegrees(value) for value in values]
        except ValueError as error:
            print(error)


def send_command(port, request):
    port.write((json.dumps(request, separators=(",", ":")) + "\n").encode())
    port.flush()
    line = port.readline()
    if not line:
        print("Nu s-a primit răspuns de la Pi (timeout).")
        return
    try:
        response = json.loads(line)
    except (UnicodeDecodeError, json.JSONDecodeError):
        print(f"Răspuns invalid: {line!r}")
        return

    if response.get("id") != request["id"]:
        print(f"ID neașteptat în răspuns: {response}")
    elif response.get("type") == "error":
        print(f"Eroare: {response['message']}")
    elif response.get("type") == "ack":
        labels = {
            "set_pwm": "PCA9685 0x42 și 0x41: comandă aplicată.",
            "turn_on_laser_led": "Laser LED aprins.",
            "turn_off_laser_led": "Laser LED stins.",
            "turn_on_led": "LED aprins.",
            "turn_off_led": "LED stins.",
        }
        print(labels.get(request["type"], "Comandă aplicată."))
    elif response.get("type") == "imu":
        values = response["values"]
        print("Accelerație (m/s²):", [v / 100 for v in values[0:3]])
        print("Giroscop (rad/s):", [v / 100 for v in values[3:6]])
        print("Magnetometru (µT):", [v / 100 for v in values[6:9]])
        print("Temperatură (°C):", values[9] / 100)
    elif response.get("type") == "stops":
        names = ("SD", "D", "FD", "SS", "S", "FS")
        for name, value in zip(names, response["values"]):
            state = "apăsat" if value == 0 else "liber"
            print(f"Endstop {name}: {state} (GPIO={value})")
    else:
        print(f"Răspuns necunoscut: {response}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--port", help="portul serial de pe laptop, de ex. /dev/ttyACM0")
    args = parser.parse_args()
    port_name = args.port or choose_port()
    if not port_name:
        parser.error("nu ai ales un port serial")

    try:
        with serial.Serial(port_name, 115200, timeout=5, write_timeout=5) as port:
            request_id = 0
            while True:
                print("\n1. Citește IMU")
                print("2. Același unghi pentru toate cele 20 de ieșiri")
                print("3. Câte un unghi pentru fiecare dintre cele 20 de ieșiri")
                print("4. Citește cele 6 endstop-uri")
                print("5. Aprinde laser LED")
                print("6. Stinge laser LED")
                print("7. Aprinde LED")
                print("8. Stinge LED")
                print("0. Ieșire")
                choice = input("Alege: ").strip()

                if choice == "0":
                    break
                if choice not in {"1", "2", "3", "4", "5", "6", "7", "8"}:
                    print("Opțiune necunoscută.")
                    continue
                request_id += 1
                if choice == "1":
                    request = {"type": "read_imu", "id": request_id}
                elif choice == "4":
                    request = {"type": "read_stops", "id": request_id}
                elif choice in {"5", "6", "7", "8"}:
                    command = {
                        "5": "turn_on_laser_led",
                        "6": "turn_off_laser_led",
                        "7": "turn_on_led",
                        "8": "turn_off_led",
                    }[choice]
                    request = {"type": command, "id": request_id}
                else:
                    if choice == "3":
                        print("Valorile 1–10: PCA 0x42, canalele 0–9;")
                        print("valorile 11–20: PCA 0x41, canalele 6–15.")
                    values = ask_angles(1 if choice == "2" else 20)
                    request = {
                        "type": "set_pwm", "id": request_id,
                        "angles": values * 20 if choice == "2" else values,
                    }
                send_command(port, request)
    except (serial.SerialException, OSError) as error:
        print(f"Portul serial nu poate fi folosit: {error}", file=sys.stderr)
        return 1
    except (KeyboardInterrupt, EOFError):
        print("\nIeșire.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
