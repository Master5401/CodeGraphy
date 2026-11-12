package audit.symbols;

public class Base {
    public int choose(int value) {
        return value;
    }

    public String choose(String value) {
        return value;
    }

    protected int adjust(int value) {
        return value + 1;
    }
}
