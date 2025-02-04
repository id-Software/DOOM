// New modular structure
namespace doom {
    class Game {
    public:
        Game();
        bool Init();
        void Tick();
        void Shutdown();
        
    private:
        std::unique_ptr<Renderer> renderer;
        std::unique_ptr<Sound> sound;
        std::unique_ptr<Input> input;
    };
}