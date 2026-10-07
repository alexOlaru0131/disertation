#pragma once

#ifdef __cplusplus

#include "stm32l452xx.h"
#include "stm32l4xx_hal.h"
#include <stdint.h>

struct Color {
  uint8_t rgb[3];
};

struct Pin {
  uint16_t pin_id;
  GPIO_TypeDef *pin_port;
};

struct Ili9488 {
  uint16_t width{480};
  uint16_t height{320};

  Pin LCD_RD;
  Pin LCD_WR;
  Pin LCD_RS;
  Pin LCD_RST;

  Pin LCD_DATA_PINS[8];

  void init_display();
  void set_pixel_color(Color);
  GPIO_PinState return_pin_state_for_data_line(uint8_t, uint8_t);
  void write_register(uint8_t, const uint8_t *, uint8_t);
  void write_data(uint8_t);
};

#endif
