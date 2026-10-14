#define configUSE_PREEMPTION 1
#define configUSE_TIME_SLICING 1
#define configCPU_CLOCK_HZ 48000000UL
#define configTICK_RATE_HZ 1000
#define configMAX_PRIORITIES 5
#define configMINIMAL_STACK_SIZE 128
#define configTOTAL_HEAP_SIZE (4 * 1024)
#define configMAX_TASK_NAME_LEN 16
#define configUSE_16_BIT_TICKS 0
#define configSUPPORT_DYNAMIC_ALLOCATION 1
#define configSUPPORT_STATIC_ALLOCATION 0
#define configUSE_MUTEXES 1
#define configUSE_TIMERS 0
#define configUSE_CO_ROUTINES 0
#define INCLUDE_vTaskDelay 1
#define INCLUDE_vTaskDelete 1
#define configENABLE_MPU 0
#define configUSE_IDLE_HOOK 0
#define configUSE_TICK_HOOK 0

#define vPortSVCHandler SVC_Handler
#define xPortPendSVHandler PendSV_Handler
#define xPortSysTickHandler SysTick_Handler
