#include "color.h"
#include "stm32l452xx.h"
#include "stm32l4xx_hal.h"
#include <stdint.h>

struct DrawingType {
  uint8_t drawing_operation;
  uint16_t x1, x2, y1, y2;
  struct Color color;
};
