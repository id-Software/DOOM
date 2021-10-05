#ifndef __I_INPUT__
#define __I_INPUT__
#include "doomtype.h"
#include <SFML/Graphics.h>
float mouseEvX, mouseEvY;
extern boolean mousebuttons[3];
void I_HandleMouse(sfRenderWindow* window, float windowScale);
#endif