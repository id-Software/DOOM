class InputSystem {
public:
    void Init();
    void Update();
    
    // Support for modern input devices
    bool IsKeyPressed(int key);
    float GetMouseX();
    float GetMouseY();
    
    // Add gamepad support
    bool HasGamepad();
    float GetAxisValue(int axis);
    bool IsButtonPressed(int button);
    
private:
    void ProcessEvents();
    void UpdateGamepad();
};