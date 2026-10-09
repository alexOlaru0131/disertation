#ifdef __cplusplus

#include "ili9488.hpp"
#include "main.h"
#include "stm32l4xx_hal.h"
#include "stm32l4xx_hal_gpio.h"

extern "C" {

void Ili9488::write_data(uint8_t data) {
  for (int i = 0; i < 8; i++) {
    HAL_GPIO_WritePin(this->LCD_DATA_PINS[i].pin_port,
                      this->LCD_DATA_PINS[i].pin_id,
                      return_pin_state_for_data_line(i, data));
  }
}

void Ili9488::write_byte(uint8_t value, GPIO_PinState data_mode) {
  HAL_GPIO_WritePin(this->LCD_RD.pin_port, this->LCD_RD.pin_id, GPIO_PIN_SET);
  HAL_GPIO_WritePin(this->LCD_RS.pin_port, this->LCD_RS.pin_id, data_mode);
  this->write_data(value);

  HAL_GPIO_WritePin(this->LCD_WR.pin_port, this->LCD_WR.pin_id, GPIO_PIN_RESET);
  HAL_GPIO_WritePin(this->LCD_WR.pin_port, this->LCD_WR.pin_id, GPIO_PIN_SET);
}

void Ili9488::write_register(uint8_t command, const uint8_t *args,
                             uint8_t args_count) {
  if (args_count > 0 && args == nullptr) {
    return;
  }

  this->write_byte(command, GPIO_PIN_RESET);

  for (int i = 0; i < args_count; i++) {
    this->write_byte(args[i], GPIO_PIN_SET);
  }
}

void Ili9488::set_drawing_window(uint16_t x0, uint16_t y0, uint16_t x1,
                                 uint16_t y1) {
  const uint8_t column_args[] = {
      static_cast<uint8_t>(x0 >> 8), static_cast<uint8_t>(x0 & 0xFF),
      static_cast<uint8_t>(x1 >> 8), static_cast<uint8_t>(x1 & 0xFF)};
  const uint8_t row_args[] = {
      static_cast<uint8_t>(y0 >> 8), static_cast<uint8_t>(y0 & 0xFF),
      static_cast<uint8_t>(y1 >> 8), static_cast<uint8_t>(y1 & 0xFF)};

  this->write_register(0x2A, column_args,
                       static_cast<uint8_t>(sizeof(column_args)));
  this->write_register(0x2B, row_args, static_cast<uint8_t>(sizeof(row_args)));
}

void Ili9488::write_color_data(Color color) {
  this->write_byte(((color.rgb[0] & 0x1F) << 3) | ((color.rgb[1] >> 3) & 0x07),
                   GPIO_PIN_SET);
  this->write_byte(((color.rgb[1] & 0x07) << 5) | (color.rgb[2] & 0x1F),
                   GPIO_PIN_SET);
}

void Ili9488::init_display() {
  this->LCD_RD = Pin{LCD_RD_Pin, LCD_RD_GPIO_Port};
  this->LCD_WR = Pin{LCD_WR_Pin, LCD_WR_GPIO_Port};
  this->LCD_RS = Pin{LCD_RS_Pin, LCD_RS_GPIO_Port};
  this->LCD_RST = Pin{LCD_RST_Pin, LCD_RST_GPIO_Port};

  this->LCD_DATA_PINS[0] = Pin{LCD_D0_Pin, LCD_D0_GPIO_Port};
  this->LCD_DATA_PINS[1] = Pin{LCD_D1_Pin, LCD_D1_GPIO_Port};
  this->LCD_DATA_PINS[2] = Pin{LCD_D2_Pin, LCD_D2_GPIO_Port};
  this->LCD_DATA_PINS[3] = Pin{LCD_D3_Pin, LCD_D3_GPIO_Port};
  this->LCD_DATA_PINS[4] = Pin{LCD_D4_Pin, LCD_D4_GPIO_Port};
  this->LCD_DATA_PINS[5] = Pin{LCD_D5_Pin, LCD_D5_GPIO_Port};
  this->LCD_DATA_PINS[6] = Pin{LCD_D6_Pin, LCD_D6_GPIO_Port};
  this->LCD_DATA_PINS[7] = Pin{LCD_D7_Pin, LCD_D7_GPIO_Port};

  HAL_GPIO_WritePin(this->LCD_RD.pin_port, this->LCD_RD.pin_id, GPIO_PIN_SET);
  HAL_GPIO_WritePin(this->LCD_WR.pin_port, this->LCD_WR.pin_id, GPIO_PIN_SET);
  HAL_GPIO_WritePin(this->LCD_RS.pin_port, this->LCD_RS.pin_id, GPIO_PIN_SET);

  HAL_GPIO_WritePin(this->LCD_RST.pin_port, this->LCD_RST.pin_id,
                    GPIO_PIN_RESET);
  HAL_Delay(20);
  HAL_GPIO_WritePin(this->LCD_RST.pin_port, this->LCD_RST.pin_id, GPIO_PIN_SET);
  HAL_Delay(120);

  this->write_register(0x01, nullptr, 0);
  HAL_Delay(120);
  this->write_register(0x28, nullptr, 0);

  const uint8_t positive_gamma[] = {0x00, 0x03, 0x09, 0x08, 0x16,
                                    0x0A, 0x3F, 0x78, 0x4C, 0x09,
                                    0x0A, 0x08, 0x16, 0x1A, 0x0F};
  const uint8_t negative_gamma[] = {0x00, 0x16, 0x19, 0x03, 0x0F,
                                    0x05, 0x32, 0x45, 0x46, 0x04,
                                    0x0E, 0x0D, 0x35, 0x37, 0x0F};
  const uint8_t power_control_1[] = {0x17, 0x15};
  const uint8_t power_control_2[] = {0x41};
  const uint8_t vcom_control[] = {0x00, 0x12, 0x80};
  const uint8_t memory_access[] = {0x28};
  const uint8_t pixel_format[] = {0x55};
  const uint8_t interface_mode[] = {0x00};
  const uint8_t frame_rate[] = {0xA0};
  const uint8_t inversion_control[] = {0x02};
  const uint8_t display_function[] = {0x02, 0x02};
  const uint8_t interface_control[] = {0x00};
  const uint8_t adjustment_control[] = {0xA9, 0x51, 0x2C, 0x82};

  this->write_register(0xE0, positive_gamma,
                       static_cast<uint8_t>(sizeof(positive_gamma)));
  this->write_register(0xE1, negative_gamma,
                       static_cast<uint8_t>(sizeof(negative_gamma)));
  this->write_register(0xC0, power_control_1,
                       static_cast<uint8_t>(sizeof(power_control_1)));
  this->write_register(0xC1, power_control_2,
                       static_cast<uint8_t>(sizeof(power_control_2)));
  this->write_register(0xC5, vcom_control,
                       static_cast<uint8_t>(sizeof(vcom_control)));
  this->write_register(0x36, memory_access,
                       static_cast<uint8_t>(sizeof(memory_access)));
  this->write_register(0x3A, pixel_format,
                       static_cast<uint8_t>(sizeof(pixel_format)));
  this->write_register(0xB0, interface_mode,
                       static_cast<uint8_t>(sizeof(interface_mode)));
  this->write_register(0xB1, frame_rate,
                       static_cast<uint8_t>(sizeof(frame_rate)));
  this->write_register(0xB4, inversion_control,
                       static_cast<uint8_t>(sizeof(inversion_control)));
  this->write_register(0xB6, display_function,
                       static_cast<uint8_t>(sizeof(display_function)));
  this->write_register(0xE9, interface_control,
                       static_cast<uint8_t>(sizeof(interface_control)));
  this->write_register(0xF7, adjustment_control,
                       static_cast<uint8_t>(sizeof(adjustment_control)));

  this->write_register(0x11, nullptr, 0);
  HAL_Delay(120);
  this->write_register(0x29, nullptr, 0);
  HAL_Delay(20);
}

GPIO_PinState Ili9488::return_pin_state_for_data_line(uint8_t pin_number,
                                                      uint8_t color_value) {
  if (color_value & (1 << pin_number))
    return GPIO_PIN_SET;
  else
    return GPIO_PIN_RESET;
}

void Ili9488::draw_pixel(uint16_t x, uint16_t y, Color color) {
  if (x >= this->width || y >= this->height) {
    return;
  }

  this->set_drawing_window(x, y, x, y);
  this->write_register(0x2C, nullptr, 0);
  this->write_color_data(color);
}

void Ili9488::fill_screen(Color color) {
  this->set_drawing_window(0, 0, this->width - 1, this->height - 1);
  this->write_register(0x2C, nullptr, 0);

  const uint32_t pixel_count =
      static_cast<uint32_t>(this->width) * this->height;
  for (uint32_t i = 0; i < pixel_count; i++) {
    this->write_color_data(color);
  }
}

void Ili9488::draw_image_in_window(uint16_t x0, uint16_t y0, uint16_t x1,
                                   uint16_t y1, Image image) {
  this->set_drawing_window(x0, y0, x1 - 1, y1 - 1);
  this->write_register(0x2C, nullptr, 0);

  for (int i = 0; i < y1 - y0; i++) {
    for (int j = 0; j < x1 - x0; j++) {
      this->write_color_data(image.pixel[i][j]);
    }
  }
}
}

#endif
