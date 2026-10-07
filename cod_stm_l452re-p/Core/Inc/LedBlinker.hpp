#pragma once

#include "stm32l4xx_hal.h"

class LedBlinker {
public:
  LedBlinker(GPIO_TypeDef *port_ptr, uint16_t pin_num)
      : port(port_ptr), pin(pin_num) {}

  void on() { HAL_GPIO_WritePin(port, pin, GPIO_PIN_SET); }
  void off() { HAL_GPIO_WritePin(port, pin, GPIO_PIN_RESET); }

private:
  GPIO_TypeDef *port;
  uint16_t pin;
};
