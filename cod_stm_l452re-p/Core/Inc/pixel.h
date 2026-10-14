#include "color.h"
#include "stm32l452xx.h"
#include "stm32l4xx_hal.h"
#include <stdint.h>

struct Pixel {
  uint16_t x;
  uint16_t y;
  struct Color color;
};
