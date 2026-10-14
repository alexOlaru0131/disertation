#ifndef PCA9685_HPP
#define PCA9685_HPP

#include <stdint.h>

struct Pca9685 {
  uint8_t number_of_servos;
  uint8_t address;

  int32_t init_device();
  int32_t set_pwm_vals(const uint32_t *);
};

#endif
