#include "pca9685.hpp"
#include "FreeRTOS.h"
#include "driver_init.h"
#include "hal_i2c_m_sync.h"
#include "pca9685_c.h"
#include "task.h"

#define PCA9685_MODE1 0x00
#define PCA9685_MODE2 0x01
#define PCA9685_LED0_ON_L 0x06
#define PCA9685_PRE_SCALE 0xFE
#define PCA9685_PWM_FREQ_REG_VAL 0x79
#define PCA9685_MODE1_RESTART 0x80
#define PCA9685_MODE1_AI 0x20
#define PCA9685_MODE1_SLEEP 0x10
#define PCA9685_MODE2_OUTDRV 0x04

#define MIN_US 500
#define MAX_US 2400

static int32_t write_bytes(uint8_t address, uint8_t *bytes, uint8_t length) {
  struct _i2c_m_msg message;
  message.addr = address;
  message.flags = I2C_M_STOP;
  message.len = length;
  message.buffer = bytes;
  return i2c_m_sync_transfer(&I2C_M_SYNC_SERCOM5, &message);
}

static int32_t write_register(uint8_t address, uint8_t reg, uint8_t value) {
  uint8_t bytes[2] = {reg, value};
  return write_bytes(address, bytes, sizeof(bytes));
}

static int32_t read_register(uint8_t address, uint8_t reg, uint8_t *value) {
  (void)i2c_m_sync_set_slaveaddr(&I2C_M_SYNC_SERCOM5, address, I2C_M_SEVEN);
  int32_t result = i2c_m_sync_cmd_read(&I2C_M_SYNC_SERCOM5, reg, value, 1);
  if (result != 0)
    (void)i2c_m_sync_send_stop(&I2C_M_SYNC_SERCOM5);
  return result;
}

int32_t Pca9685::init_device() {
  uint8_t old_mode;
  uint8_t old_mode2;
  int32_t return_val = read_register(this->address, PCA9685_MODE1, &old_mode);
  if (return_val != 0)
    return return_val;

  return_val = read_register(this->address, PCA9685_MODE2, &old_mode2);
  if (return_val != 0)
    return return_val;

  return_val = write_register(this->address, PCA9685_MODE2,
                              old_mode2 | PCA9685_MODE2_OUTDRV);
  if (return_val != 0)
    return return_val;

  const uint8_t sleep_mode =
      (old_mode & (uint8_t)~PCA9685_MODE1_RESTART) | PCA9685_MODE1_SLEEP;
  return_val = write_register(this->address, PCA9685_MODE1, sleep_mode);
  if (return_val != 0)
    return return_val;

  return_val = write_register(this->address, PCA9685_PRE_SCALE,
                              PCA9685_PWM_FREQ_REG_VAL);
  if (return_val != 0)
    return return_val;

  const uint8_t wake_mode =
      old_mode & (uint8_t)~(PCA9685_MODE1_SLEEP | PCA9685_MODE1_RESTART);
  return_val = write_register(this->address, PCA9685_MODE1, wake_mode);
  if (return_val != 0)
    return return_val;

  /* hal_delay's delay_ms reprograms SysTick, which FreeRTOS uses for its tick. */
  vTaskDelay(pdMS_TO_TICKS(5));
  return_val = write_register(this->address, PCA9685_MODE1,
                              wake_mode | PCA9685_MODE1_RESTART |
                                  PCA9685_MODE1_AI);
  if (return_val != 0)
    return return_val;

  return 0;
}

static uint16_t angle_to_ticks(float degrees, float min_us, float max_us) {
  if (degrees < 0.0f)
    degrees = 0.0f;
  if (degrees > 180.0f)
    degrees = 180.0f;

  const float pulse_us = min_us + (degrees / 180.0f) * (max_us - min_us);

  return static_cast<uint16_t>(pulse_us * 50 * 4096.0f / 1000000.0f + 0.5f);
}

int32_t Pca9685::set_pwm_vals(const uint32_t *pwm_degrees) {
  for (uint8_t i = 0; i < this->number_of_servos; i++) {
    uint16_t bits_value = angle_to_ticks(pwm_degrees[i], MIN_US, MAX_US);
    uint8_t bytes[5] = {static_cast<uint8_t>(PCA9685_LED0_ON_L + 4 * i),
                        0, 0, static_cast<uint8_t>(bits_value & 0xFF),
                        static_cast<uint8_t>((bits_value >> 8) & 0x0F)};
    int32_t result = write_bytes(this->address, bytes, sizeof(bytes));
    if (result != 0)
      return result;
  }

  return 0;
}

extern "C" void servos_task(void *argument) {
  (void)argument;
  Pca9685 driver1{9, 0x41};
  Pca9685 driver2{9, 0x42};

  /* D13 (PA17) indicates whether the task reaches I2C and the PCA9685s reply. */
  gpio_set_pin_level(PA17, true);
  vTaskDelay(pdMS_TO_TICKS(100));
  gpio_set_pin_level(PA17, false);

  if (i2c_m_sync_enable(&I2C_M_SYNC_SERCOM5) != 0) {
    for (;;) {
      gpio_set_pin_level(PA17, true);
      vTaskDelay(pdMS_TO_TICKS(100));
      gpio_set_pin_level(PA17, false);
      vTaskDelay(pdMS_TO_TICKS(900));
    }
  }

  bool driver1_ready = false;
  bool driver2_ready = false;

  uint32_t pwm_vals[9];
  int32_t angle = 30;
  int32_t step = 2;
  uint32_t led_tick = 0;

  for (;;) {
    if (!driver1_ready)
      driver1_ready = driver1.init_device() == 0;
    if (!driver2_ready)
      driver2_ready = driver2.init_device() == 0;

    for (uint8_t i = 0; i < 9; i++)
      pwm_vals[i] = (uint32_t)angle;

    if (driver1_ready && driver1.set_pwm_vals(pwm_vals) != 0)
      driver1_ready = false;
    if (driver2_ready && driver2.set_pwm_vals(pwm_vals) != 0)
      driver2_ready = false;

    /* Steady: both drivers OK; slow blink: one; fast blink: neither. */
    if (driver1_ready && driver2_ready)
      gpio_set_pin_level(PA17, true);
    else if (driver1_ready || driver2_ready)
      gpio_set_pin_level(PA17, (led_tick / 10) % 2 == 0);
    else
      gpio_set_pin_level(PA17, (led_tick / 3) % 2 == 0);
    led_tick++;

    if (angle >= 150)
      step = -2;
    else if (angle <= 30)
      step = 2;
    angle += step;

    vTaskDelay(pdMS_TO_TICKS(50));
  }
}
