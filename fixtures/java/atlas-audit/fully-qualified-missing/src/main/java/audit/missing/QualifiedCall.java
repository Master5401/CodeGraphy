package audit.missing;

public class QualifiedCall {
    public Object convert(Object value) {
        return absent.platform.Tools.normalize(value);
    }
}
