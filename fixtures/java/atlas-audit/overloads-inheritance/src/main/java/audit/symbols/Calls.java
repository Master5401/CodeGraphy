package audit.symbols;

public class Calls {
    private final ValueProvider provider = new Child();

    public int run(int value) {
        return provider.provide(value);
    }
}
