#include "i_input.h"
#include "d_main.h"
#include "doomstat.h"
#include "neapolitan.h"
#include <SFML/System.h>
#include <stdio.h>

unsigned int mouse_state = 0;
boolean mouseLocked = false;    
int oldmouseX = 0;
int oldMouseY = 0;

event_t mousebuttonevent;

float mouse_acceleration = 2.0;
int mouse_threshold = 10;
int AccelerateMouse(int val)
{
    if (val < 0)
        return -AccelerateMouse(-val);

    if (val > mouse_threshold)
    {
        return (int)((val - mouse_threshold) * mouse_acceleration + mouse_threshold);
    }
    else
    {
        return val;
    }
}

void I_HandleMouse(sfRenderWindow* window, float windowScale)
{
    if(useMouse)
    {
        sfVector2i mouse = sfMouse_getPositionRenderWindow(window);
        mouse.x /= windowScale;
        mouse.y /= windowScale;

        int tempx = oldmouseX;
        int tempy = oldMouseY;
        oldmouseX = mouse.x;
        oldMouseY = mouse.y;

        mouse.x -= tempx;
        mouse.y -= tempy;

        if(mouse.x != 0 && mouse.y != 0)
        {
            // event_t event;
            // event.type = ev_mouse;
            // event.data1 = mouse_state;
            // event.data2 = AccelerateMouse(mouse.x);
            // event.data3 = AccelerateMouse(mouse.y);
            // D_PostEvent(&event);
            mouseEvX = AccelerateMouse(mouse.x);
            mouseEvY = AccelerateMouse(mouse.y);
        }


        if(mouseLocked)
        {
            sfRenderWindow_setMouseCursorVisible(window, !useMouse);
            //lock mouse
            if(useMouse)
            {
                sfVector2u windowsize = sfRenderWindow_getSize(window);
                sfVector2i lockedPosition;
                lockedPosition.x = windowsize.x/2;
                lockedPosition.y = windowsize.y/2;
                oldmouseX = lockedPosition.x;
                oldMouseY = lockedPosition.y;
                sfMouse_setPositionRenderWindow(lockedPosition, window);
            }
        }
    }
}

void I_Click(sfMouseButton button, boolean down)
{
    int buttonbit;
    switch (button)
    {
    case sfMouseLeft:
        buttonbit = 0;
        break;
    case sfMouseRight:
        buttonbit = 1;
        break;
    case sfMouseMiddle:
        buttonbit = 2;
        break;
    default:
        break;
    }

    if(down)
    {
        mouse_state |= (1 << buttonbit);
    }else
    {
        mouse_state &= ~(1 << buttonbit);
    }

    mousebuttonevent.type = ev_mouse;
    mousebuttonevent.data1 = mouse_state;
    mousebuttonevent.data2 = mousebuttonevent.data3 = 0;
    
    D_PostEvent(&mousebuttonevent);
}