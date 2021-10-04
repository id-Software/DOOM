#ifndef __I_INPUT__
#define __I_INPUT__
#include "doomtype.h"
#include <SFML/Graphics.h>
float mouseEvX, mouseEvY;

void I_HandleMouse(sfRenderWindow* window, float windowScale);
void I_Click(sfMouseButton button, boolean down);
#endif