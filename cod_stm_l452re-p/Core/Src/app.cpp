
#include "app.hpp"
#include "LedBlinker.hpp"
#include "frame.hpp"
#include "ili9488.hpp"

extern "C" {
#include "cmsis_os2.h"
#include "main.h"
#include "stm32l4xx_hal.h"
}

void start(osMessageQueueId_t queue) {

  LedBlinker led(GPIOB, GPIO_PIN_13);
  Ili9488 ili9488;
  ili9488.queue = queue;

  Frame frame{};
  frame.display = ili9488;

  frame.init_frame();
  frame.create_measurements_window();

  osDelay(100);

  while (1) {
    osDelay(1);
  }
}

void drawing_task_implementation(osMessageQueueId_t queue) {
  DrawingType dt{};
  Ili9488 ili9488;

  ili9488.init_display(queue);
  LedBlinker led(GPIOB, GPIO_PIN_13);

  while (1) {
    led.on();
    while (osMessageQueueGet(queue, &dt, NULL, osWaitForever) == osOK) {
      led.off();
      switch (dt.drawing_operation) {
      case 1: {
        ili9488.fill_screen(dt.color);
        break;
      }
      case 2: {
        ili9488.draw_color_in_window(dt.x1, dt.y1, dt.x2, dt.y2, dt.color);
        break;
      }
      case 3: {
        ili9488.set_drawing_window(dt.x1, dt.y1, dt.x2, dt.y2);
        break;
      }
      case 4: {
        ili9488.write_color_data(dt.color);
        break;
      }
      default:
        break;
      }
    }
  }
}
