#include <libusb-1.0/libusb.h>

#define WF_PIPESIZE 16768
#define WF_IF 0
#define WF_ISOPIPE 0x81
#define WF_VENDOR 0x9cd
#define WF_PRODUCT 0x2001

typedef struct wf_device {
        int (*process_func)(struct wf_device *, unsigned char *);
        struct libusb_device_handle *devh;
        struct libusb_transfer *xfr;
        struct libusb_transfer *ctrl_xfr;
        unsigned char buf[WF_PIPESIZE];
        unsigned char *bufptr;
        size_t callback;
} device;

typedef struct wf_ctrl_request {
    int request;
    int value;
    int index;
    unsigned char *bytes;
    int size;
    int async;
} ctrl_req;

typedef int (*process_func)(struct wf_device *, unsigned char *);

struct wf_device *wf_open(process_func func, size_t callback);
void wf_close(struct wf_device *);
size_t wf_callback(struct wf_device *);