#include <libusb-1.0/libusb.h>

#define WF_PIPESIZE 16768
#define WF_IF 0
#define WF_ISOPIPE 0x81
#define WF_VENDOR 0x9cd
#define WF_PRODUCT 0x2001

typedef struct wavefinder {
        int (*process_func)(struct wavefinder *, unsigned char *);
        struct libusb_device_handle *devh;
        struct libusb_transfer *xfr;
        struct libusb_transfer *ctrl_xfr;
        unsigned char buf[WF_PIPESIZE];
        unsigned char *bufptr;
} wf;

typedef struct wf_ctrl_request {
    int request;
    int value;
    int index;
    unsigned char *bytes;
    int size;
    int async;
} ctrl_req;

typedef int (*process_func)(struct wavefinder *, unsigned char *);

struct wavefinder *wf_open(process_func func);
void wf_close(struct wavefinder *);