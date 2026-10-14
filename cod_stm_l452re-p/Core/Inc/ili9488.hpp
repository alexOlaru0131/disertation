#pragma once

#ifdef __cplusplus

#include "cmsis_os2.h"
#include "drawing_type.h"
#include "stm32l452xx.h"
#include "stm32l4xx_hal.h"
#include <stdint.h>

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

  osMessageQueueId_t queue;

  void init_display(osMessageQueueId_t);

  void draw_pixel(uint16_t x, uint16_t y, Color color);

  void request_fill_screen(Color color);
  void fill_screen(Color color);

  void request_draw_color_in_window(uint16_t x0, uint16_t y0, uint16_t x1,
                                    uint16_t y1, Color color);
  void draw_color_in_window(uint16_t x0, uint16_t y0, uint16_t x1, uint16_t y1,
                            Color color);

  void request_set_drawing_window(uint16_t x0, uint16_t y0, uint16_t x1,
                                  uint16_t y1);
  void set_drawing_window(uint16_t x0, uint16_t y0, uint16_t x1, uint16_t y1);
  void request_write_color_data(Color color);
  void write_color_data(Color color);

private:
  void write_register(uint8_t command, const uint8_t *args, uint8_t args_count);
  void write_byte(uint8_t value, GPIO_PinState data_mode);
  void write_data(uint8_t);
  GPIO_PinState return_pin_state_for_data_line(uint8_t, uint8_t);
};

#endif
