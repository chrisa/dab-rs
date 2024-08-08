#include <libusb-1.0/libusb.h>
#include <stdbool.h>

#define WF_PIPESIZE 16768
#define WF_IF 0
#define WF_ISOPIPE 0x81
#define WF_VENDOR 0x9cd
#define WF_PRODUCT 0x2001

#define WF_REQ_SLMEM  3
#define WF_REQ_TUNE   4
#define WF_REQ_TIMING 5

typedef struct wf_device {
    struct libusb_device_handle *devh;
    struct libusb_transfer *xfr;
    struct libusb_transfer *ctrl_xfr;
    unsigned char buf[WF_PIPESIZE];
    unsigned char *bufptr;
    void (*callback)(struct wf_device *, void *, unsigned char *, size_t len);
    void *data;
} device;

typedef struct wf_ctrl_request {
    int request;
    int value;
    int index;
    unsigned char *bytes;
    int size;
    int async;
} ctrl_req;

typedef void (*process_func)(struct wf_device *wf, void *data, unsigned char *buf, size_t len);

struct wf_device *wf_open();
void wf_set_callback(struct wf_device *wf, process_func callback, void *data);
void wf_close(struct wf_device *wf);
size_t wf_callback(struct wf_device *wf);
size_t wf_context(struct wf_device *wf);
void wf_read(struct wf_device *wf);
void wf_handle_events();
struct wf_ctrl_request *wf_ctrl_request_init(uint32_t request, uint32_t value, uint32_t index, unsigned char *bytes, size_t size, bool async);
size_t wf_usb_ctrl_msg(struct wf_device *wf, struct wf_ctrl_request *req);