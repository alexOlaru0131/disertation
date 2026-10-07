
#include "app.hpp"
#include "LedBlinker.hpp"

extern "C" {
#include "main.h"
#include "stm32l4xx_hal.h"
}

void start() {

  LedBlinker led(GPIOB, GPIO_PIN_13); // Blue LED on Nucleo board

  while (1) { // infinite loop
    led.on();
    HAL_Delay(5000); // 5 seconds
    led.off();
    HAL_Delay(5000);
  }
}
