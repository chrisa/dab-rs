/*
    wfsl11r.h

    Copyright (C) 2005 David Crawley

    This file is part of OpenDAB.

    OpenDAB is free software: you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    OpenDAB is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with OpenDAB.  If not, see <http://www.gnu.org/licenses/>.
*/
/*
** A Cypress Semiconductor SL11R USB microcontroller is used in the WaveFinder.
** 
** This file allocates symbolic names to the register addresses
** and so forth.
**
*/

/* Output Data Register 0
**
*/
#define OUTREG0 0xc01e

#define OUTREG1 0xc024

#define IOCTRLREG1 0xc028

/* The following are concerned mainly (only?) with LED control */
/* PWM Control Register */
#define PWMCTRLREG 0xc0e6

/* PWM Maximum Count Register */
#define PWMMAXCNT 0xc0e8

/* PWM Channel 0 Start Register - Does this control anything ? */
#define PWMCH0STRT 0xc0ea

/* PWM Channel 0 Stop Register  - Does this control anything ? */
#define PWMCH0STOP 0xc0ec

/* PWM Channel 1 Start Register - Controls blue LED */
#define PWMCH1STRT 0xc0ee

/* PWM Channel 1 Stop Register  - Controls blue LED*/
#define PWMCH1STOP 0xc0f0

/* PWM Channel 2 Start Register - Controls red LED */
#define PWMCH2STRT 0xc0f2

/* PWM Channel 2 Stop Register  - Controls red LED */
#define PWMCH2STOP 0xc0f4

/* PWM Channel 3 Start Register - Controls green LED */
#define PWMCH3STRT 0xc0f6

/* PWM Channel 3 Stop Register  - Controls green LED */
#define PWMCH3STOP 0xc0f8

/* PWM Cycle Count Register */
#define PWMCYCCNT 0xc0fa



/* DAC value - probably used by the AFC systems
** to offset the reference oscillator frequency
** Note the wavefinder firmware, as far as I can see,
** always writes the same value to the DAC!
*/
#define DACVALUE 0x0366

/* Don't know what these do yet */
#define UNK0XC120 0xc120

/* USB Vendor requests (bRequest field) for the WaveFinder */

#define DSPADATA 1
#define DSPBDATA 2
