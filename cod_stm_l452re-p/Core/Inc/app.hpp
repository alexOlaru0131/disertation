#pragma once

#ifdef __cplusplus

#include "cmsis_os2.h"

extern "C" {
#endif

void start(osMessageQueueId_t);
void drawing_task_implementation(osMessageQueueId_t);

#ifdef __cplusplus
}
#endif
