#include "i_input.h"
#include "d_main.h"
#include <SFML/System.h>
#include <stdio.h>

unsigned int mouse_state = 0;
int oldmouseX = 0;
int oldMouseY = 0;

event_t mousebuttonevent;
void I_HandleMouse(sfRenderWindow* window)
{
    sfVector2i rawmouse = sfMouse_getPositionRenderWindow(window);
    sfVector2f mouse = sfRenderWindow_mapPixelToCoords(window, rawmouse, sfRenderWindow_getView(window));
    mouse.x -= oldmouseX;
    mouse.y -= oldMouseY;

    oldmouseX = mouse.x;
    oldMouseY = mouse.y;

    if(mouse.x != 0 && mouse.y != 0)
    {
        event_t event;
        event.type = ev_mouse;
        event.data1 = mouse_state;
        event.data2 = mouse.x;
        event.data3 = mouse.y;

        D_PostEvent(&event);
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