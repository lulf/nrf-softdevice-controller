#include "nrf_errno.h"

/// SDC
#include "softdevice_controller/include/sdc.h"
#include "softdevice_controller/include/sdc_hci_cmd_controller_baseband.h"
#include "softdevice_controller/include/sdc_hci_cmd_info_params.h"
#include "softdevice_controller/include/sdc_hci_cmd_le.h"
#include "softdevice_controller/include/sdc_hci_cmd_link_control.h"
#include "softdevice_controller/include/sdc_hci_cmd_status_params.h"
#include "softdevice_controller/include/sdc_hci.h"
#include "softdevice_controller/include/sdc_hci_vs.h"
#include "softdevice_controller/include/sdc_soc.h"

// MPSL
#include "mpsl/include/nrf_errno.h"
#include "mpsl/include/mpsl_clock.h"
#include "mpsl/include/mpsl_coex.h"
#include "mpsl/include/mpsl_cx_abstract_interface.h"
#include "mpsl/include/mpsl.h"
#include "mpsl/include/mpsl_temp.h"
#include "mpsl/include/mpsl_timeslot.h"
#include "mpsl/include/mpsl_tx_power.h"
#include "mpsl/include/protocol/mpsl_cx_protocol_api.h"
#include "mpsl/include/protocol/mpsl_dppi_protocol_api.h"
