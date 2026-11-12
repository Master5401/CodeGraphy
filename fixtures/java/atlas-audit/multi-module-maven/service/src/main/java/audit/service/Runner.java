package audit.service;

import audit.api.Values;

public class Runner {
    public int run(int value) {
        return new Values().increment(value);
    }
}
