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

void Ili9488::write_register(uint8_t register_address,
                             const uint8_t *register_args = nullptr,
                             uint8_t args_count = 0) {
  HAL_GPIO_WritePin(this->LCD_RS.pin_port, this->LCD_RS.pin_id, GPIO_PIN_RESET);
  HAL_GPIO_WritePin(this->LCD_RD.pin_port, this->LCD_RD.pin_id, GPIO_PIN_SET);
  HAL_GPIO_WritePin(this->LCD_WR.pin_port, this->LCD_WR.pin_id, GPIO_PIN_SET);

  this->write_data(register_address);

  HAL_Delay(1);

  HAL_GPIO_WritePin(this->LCD_RS.pin_port, this->LCD_RS.pin_id, GPIO_PIN_SET);

  for (int i = 0; i < args_count; i++) {
    this->write_data(register_args[i]);
  }
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

  HAL_GPIO_WritePin(this->LCD_RST.pin_port, this->LCD_RST.pin_id,
                    GPIO_PIN_RESET);
  HAL_Delay(20);
  HAL_GPIO_WritePin(this->LCD_RST.pin_port, this->LCD_RST.pin_id, GPIO_PIN_SET);

  uint8_t *args;

  this->write_register(0x01, args, 0);
  HAL_Delay(20);
  this->write_register(0x28, args, 0);

  args[0] = 0x55;
  this->write_register(0x3A, args, 1);

  args[0] = 0x00;
  this->write_register(0xB0, args, 1);

  args[0] = 0x02;
  args[1] = 0x00;
  args[2] = 0x00;
  args[3] = 0x00;
  this->write_register(0xB3, args, 4);

  args[0] = 0x00;
  this->write_register(0xB4, args, 1);

  args[0] = 0x07;
  args[1] = 0x42;
  args[2] = 0x18;
  this->write_register(0xD0, args, 3);

  args[0] = 0x00;
  args[1] = 0x07;
  args[2] = 0x10;
  this->write_register(0xD1, args, 3);

  args[0] = 0x01;
  args[1] = 0x02;
  this->write_register(0xD2, args, 2);

  args[0] = 0x01;
  args[1] = 0x02;
  this->write_register(0xD3, args, 2);

  args[0] = 0x01;
  args[1] = 0x02;
  this->write_register(0xD4, args, 2);

  args[0] = 0x12;
  args[1] = 0x3B;
  args[2] = 0x00;
  args[3] = 0x02;
  args[4] = 0x11;
  this->write_register(0xC0, args, 5);

  args[0] = 0x10;
  args[1] = 0x10;
  args[2] = 0x88;
  this->write_register(0xC1, args, 3);

  args[0] = 0x03;
  this->write_register(0xC5, args, 1);

  args[0] = 0x02;
  this->write_register(0xC6, args, 1);

  args[0] = 0x00;
  args[1] = 0x32;
  args[2] = 0x36;
  args[3] = 0x45;
  args[4] = 0x06;
  args[5] = 0x16;
  args[6] = 0x37;
  args[7] = 0x75;
  args[8] = 0x77;
  args[9] = 0x54;
  args[10] = 0x0C;
  args[11] = 0x00;
  this->write_register(0xC8, args, 12);

  args[0] = 0x00;
  this->write_register(0xCC, args, 1);

  this->write_register(0x11, args, 0);
  HAL_Delay(20);
  this->write_register(0x29, args, 0);

  delete args;
}

GPIO_PinState Ili9488::return_pin_state_for_data_line(uint8_t pin_number,
                                                      uint8_t color_value) {
  if (color_value & (1 << pin_number))
    return GPIO_PIN_SET;
  else
    return GPIO_PIN_RESET;
}

void Ili9488::set_pixel_color(Color color) {
  HAL_GPIO_WritePin(this->LCD_RD.pin_port, this->LCD_RD.pin_id, GPIO_PIN_SET);
  HAL_GPIO_WritePin(this->LCD_WR.pin_port, this->LCD_WR.pin_id, GPIO_PIN_SET);
  HAL_GPIO_WritePin(this->LCD_RS.pin_port, this->LCD_RS.pin_id, GPIO_PIN_SET);

  for (int i = 0; i < 3; i++) {
    HAL_GPIO_WritePin(this->LCD_WR.pin_port, this->LCD_WR.pin_id,
                      GPIO_PIN_RESET);

    this->write_data(color.rgb[i]);

    HAL_GPIO_WritePin(this->LCD_WR.pin_port, this->LCD_WR.pin_id, GPIO_PIN_SET);
  }

  HAL_GPIO_WritePin(this->LCD_RS.pin_port, this->LCD_RS.pin_id, GPIO_PIN_RESET);
  HAL_GPIO_WritePin(this->LCD_RD.pin_port, this->LCD_RD.pin_id, GPIO_PIN_RESET);
}
}

#endif
