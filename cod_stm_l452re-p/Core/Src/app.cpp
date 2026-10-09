
#include "app.hpp"
#include "LedBlinker.hpp"
#include "ili9488.hpp"

extern "C" {
#include "cmsis_os2.h"
#include "main.h"
#include "stm32l4xx_hal.h"
}

void start() {

  LedBlinker led(GPIOB, GPIO_PIN_13);
  Ili9488 ili9488;
  ili9488.init_display();

  const Color red{{255, 0, 0}};
  const Color green{{0, 255, 0}};
  const Color blue{{0, 0, 255}};
  const Color black{0, 0, 0};

  ili9488.fill_screen(black);

  Color **image_pixels = new Color *[5];
  image_pixels[0] = new Color[3]{red, red, red};
  image_pixels[1] = new Color[3]{red, red, red};
  image_pixels[2] = new Color[3]{red, red, red};
  image_pixels[3] = new Color[3]{red, red, red};
  image_pixels[4] = new Color[3]{green, green, green};

  const Image image1{image_pixels};

  while (1) {

    ili9488.draw_image_in_window(100, 100, 103, 105, image1);
    osDelay(1);
  }
}
