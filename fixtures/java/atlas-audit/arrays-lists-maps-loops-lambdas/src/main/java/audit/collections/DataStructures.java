package audit.collections;

import java.util.HashMap;
import java.util.List;
import java.util.Map;

public class DataStructures {
    public int calculate(int x, int y) {
        int[] values = new int[2];
        values[0] = x * y;
        values[1] = values[0] + y;
        return values[1];
    }

    public Map<String, Integer> summarize(List<String> values) {
        Map<String, Integer> result = new HashMap<>();
        for (String value : values) {
            if (value != null) {
                result.put(value, value.length());
            }
        }
        values.stream()
                .map(String::trim)
                .forEach(value -> result.put(value, result.getOrDefault(value, 0) + 1));
        return result;
    }
}
