package audit.client;

import audit.provider.Converter;

public class ApiCaller {
    public String convert(String value) {
        return Converter.convert(value);
    }
}
