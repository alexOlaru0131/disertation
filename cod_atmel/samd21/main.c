#include "main.h"
#include "pca9685_c.h"

int main(void) {
  atmel_start_init();

  gpio_set_pin_function(PA17, GPIO_PIN_FUNCTION_OFF);
  gpio_set_pin_direction(PA17, GPIO_DIRECTION_OUT);
  gpio_set_pin_level(PA17, false);

  if (xTaskCreate(servos_task, "servos", 256, NULL, 1, NULL) == pdPASS) {
    vTaskStartScheduler();
  }

  for (;;) {
    gpio_toggle_pin_level(PA17);
    delay_ms(750);
  }
}
