#ifndef __I_INPUT__
#define __I_INPUT__
#include "doomtype.h"
#include <SFML/Graphics.h>
void I_HandleMouse(sfRenderWindow* window);
void I_Click(sfMouseButton button, boolean down);
#endif