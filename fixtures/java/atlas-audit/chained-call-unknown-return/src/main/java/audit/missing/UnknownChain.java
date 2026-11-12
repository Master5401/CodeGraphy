package audit.missing;

public class UnknownChain {
    public Object finish(Object input) {
        return UnknownFactory.open(input).normalize();
    }
}
