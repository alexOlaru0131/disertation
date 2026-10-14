#include "ili9488.hpp"
#ifdef __cplusplus

#include "frame.hpp"

void Frame::init_frame() {
  this->frame_height = this->display.height / 4;
  this->frame_width = this->display.width / 8;

  this->pixels = new Color *[this->frame_height] {};
  for (uint16_t i = 0; i < this->frame_height; i++)
    this->pixels[i] = new Color[this->frame_width]{};
}

void Frame::create_measurements_window() {
  this->display.request_fill_screen(BLACK);

  for (uint16_t i = 0; i < this->frame_height; i++) {
    for (uint16_t j = 0; j < this->frame_width; j++) {
      pixels[i][j] = GRAY_1;
    }
  }

  for (uint16_t i = 0; i < this->frame_height; i++) {
    pixels[i][this->frame_width - 1] = GRAY_2;
  }

  pixels[20][20] = BLACK;

  this->display.request_set_drawing_window(0, 0, this->display.width / 2,
                                           this->display.height);

  for (uint16_t i = 0; i < this->display.height; i++) {
    for (uint16_t j = 0; j < this->display.width / 2; j++) {
      this->display.request_write_color_data(pixels[i / 4][j / 4]);
    }
  }
}

#endif
