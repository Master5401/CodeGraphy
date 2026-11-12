package proof;

public final class PriceFlow {
    public int total(int quantity, int unitPrice) {
        int subtotal = quantity * unitPrice;
        return addFee(subtotal);
    }

    private int addFee(int amount) {
        int fee = 2;
        return amount + fee;
    }
}
