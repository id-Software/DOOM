#include <SFML/Window.h>
#include <cjson/cJSON.h>
#include <stdio.h>
#include <unistd.h>
#include "neapolitan.h"
#include "m_keybinds.h"

char bindnames[7][32] = {"Forward", "Backwards", "Left", "Right", "Use", "Fire", "Run"};
boolean useMouse = 0;
int keybindCount = 7;
//typing this out really makes me miss C#
char keynames[sfKeyCount][16] =
    {
        "A",
        "B",
        "C",
        "D",
        "E",
        "F",
        "G",
        "H",
        "I",
        "J",
        "K",
        "L",
        "M",
        "N",
        "O",
        "P",
        "Q",
        "R",
        "S",
        "T",
        "U",
        "V",
        "W",
        "X",
        "Y",
        "Z",
        "0",
        "1",
        "2",
        "3",
        "4",
        "5",
        "6",
        "7",
        "8",
        "9",
        "Escape", //unbindable
        "LControl",
        "LShift",
        "LAlt",
        "LSystem", //unbindable
        "RControl",
        "RShift",
        "RAlt",
        "RSystem",
        "Menu",
        "[",
        "]",
        ";",
        ",",
        ".",
        "\'",
        "/",
        "\\",
        "~",
        "=",
        "-",
        "Space",
        "Enter",
        "Backspace",
        "Tab",
        "Page Up",
        "Page Down",
        "End",
        "Home",
        "Insert",
        "Delete",
        "+",
        "-",
        "*",
        "/",
        "Left",
        "Right",
        "Up",
        "Down"

};
int unbindablekeys[] =
    {
        sfKeyEscape,
        sfKeyLSystem,
        sfKeyRSystem,
        sfKeyReturn
};

int keybinds[] =
{
        KEY_UPARROW,
        KEY_DOWNARROW,
        KEY_LEFTARROW,
        KEY_RIGHTARROW,
        KEY_RCTRL,
        KEY_RSHIFT,
        KEY_SPACE
};

int unbindableKeyCount = 4;
int snd_DoPitchShift;

boolean fixInfiniteMonsterHeight = 1;

void N_WriteConfig()
{
    printf("writing config...\n");
    cJSON* json = cJSON_CreateObject();

    cJSON_AddNumberToObject(json, "key_up", key_up);
    cJSON_AddNumberToObject(json, "key_down", key_down);
    cJSON_AddNumberToObject(json, "key_left", key_left);
    cJSON_AddNumberToObject(json, "key_right", key_right);
    cJSON_AddNumberToObject(json, "key_fire", key_fire);
    cJSON_AddNumberToObject(json, "key_use", key_use);
    cJSON_AddNumberToObject(json, "key_run", key_speed);
    cJSON_AddBoolToObject(json, "mouse_look", useMouse);
    cJSON_AddBoolToObject(json, "monster_height_fix", fixInfiniteMonsterHeight);
    char* text = cJSON_Print(json);

    FILE* file = fopen(NEAPOLITAN_SAVEFILE, "w");
    fwrite(text, strlen(text), 1, file);
    fclose(file);
}


void N_RebindKeys()
{
    key_up = keybinds[0];
    key_down = keybinds[1];
    key_left = keybinds[2];
    key_right = keybinds[3];
    key_fire = keybinds[4];
    key_speed = keybinds[5];
    key_use = keybinds[6];
}

void N_LoadConfig(void)
{
    if(access(NEAPOLITAN_SAVEFILE, F_OK))
    {
        N_WriteConfig();
        return;
    }
    printf("N_LoadConfig: loading %s\n", NEAPOLITAN_SAVEFILE);
    char textbuffer[2048];
    FILE* file = fopen(NEAPOLITAN_SAVEFILE, "r");
    fread(&textbuffer, 2048, 1, file);
    fclose(file);
    cJSON* json = cJSON_Parse(textbuffer);

    keybinds[0] = cJSON_GetObjectItem(json, "key_up")->valueint;
    keybinds[1] = cJSON_GetObjectItem(json, "key_down")->valueint;
    keybinds[2] = cJSON_GetObjectItem(json, "key_left")->valueint;
    keybinds[3] = cJSON_GetObjectItem(json, "key_right")->valueint;
    keybinds[4] = cJSON_GetObjectItem(json, "key_fire")->valueint;
    keybinds[5] = cJSON_GetObjectItem(json, "key_run")->valueint;
    keybinds[6] = cJSON_GetObjectItem(json, "key_use")->valueint;

    useMouse = cJSON_IsTrue(cJSON_GetObjectItem(json, "mouse_look"));
    fixInfiniteMonsterHeight =  cJSON_IsTrue(cJSON_GetObjectItem(json, "monster_height_fix"));

    N_RebindKeys();
}


void N_MonsterHeightFix(int choice)
{
    fixInfiniteMonsterHeight = choice;
}