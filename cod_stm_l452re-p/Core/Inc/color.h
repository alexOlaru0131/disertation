#include "stm32l452xx.h"
#include "stm32l4xx_hal.h"
#include <stdint.h>

struct Color {
  uint8_t rgb[3];
};

static const struct Color BLACK = {{0, 0, 0}};
static const struct Color GRAY_1 = {{50, 40, 50}};
static const struct Color GRAY_2 = {{40, 20, 40}};
static const struct Color RED = {{255, 0, 0}};
