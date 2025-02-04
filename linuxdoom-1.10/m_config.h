class Config {
public:
    static void LoadFromFile(const char* filename);
    static void SaveToFile(const char* filename);
    
    // Runtime configuration
    struct {
        int screen_width;
        int screen_height;
        bool fullscreen;
        bool vsync;
        std::string language;
        float mouse_sensitivity;
    } video;
    
    struct {
        bool enable_sound;
        int sample_rate;
        float music_volume;
        float sfx_volume;
    } audio;
};