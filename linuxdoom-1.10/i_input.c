#include "i_input.h"
#include "d_main.h"
#include <SFML/System.h>

unsigned int mouse_state = 0;

void I_HandleMouse(sfRenderWindow* window)
{
    sfVector2i mouse = sfMouse_getPositionRenderWindow(window);

    if(mouse.x != 0 && mouse.y != 0)
    {
        event_t event;
        event.type = ev_mouse;
        event.data2 = mouse.x;
        event.data3 = mouse.y;

        D_PostEvent(&event);
    }
}

void I_Click(sfMouseButton button)
{
    event_t event;
    event.type = ev_mouse;
    switch (button)
    {
    case sfMouseLeft:
        event.data1 = 0;
        break;
    
    default:
        break;
    }
}