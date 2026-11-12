package audit.symbols;

public class Child extends Base implements ValueProvider {
    @Override
    public int provide(int value) {
        return adjust(value);
    }

    public int chooseInteger(int value) {
        return choose(value);
    }

    public String chooseText(String value) {
        return choose(value);
    }
}
