#include "ili9488.hpp"
#include <stdint.h>

#ifdef __cplusplus
#include "ili9488.hpp"

struct Frame {
  Color **pixels;
  Ili9488 display;

  uint16_t frame_height;
  uint16_t frame_width;

  void init_frame();
  void create_measurements_window();
};

#endif
