package audit.tour;

import a.*;
import c.*;

public class Tour {
    public int chessTour(int x, int y) {
        int moves = a.KnightTour.start(x, y);
        return c.Formatter.format(moves);
    }
}
