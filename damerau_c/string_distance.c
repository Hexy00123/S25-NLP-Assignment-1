#include <stdio.h>
#include <string.h>

int min(int a, int b) {
    return a < b ? a : b;
}

int distance(const char* a, const char* b) {
    int len_a = strlen(a);
    int len_b = strlen(b);
    int d[len_a + 1][len_b + 1];

    for (int i = 0; i < len_a + 1; ++i) {
        for (int j = 0; j < len_b + 1; ++j) {
            d[i][j] = 0;
        }
    }

    for (int i = 1; i < len_a + 1; ++i) {
        for (int j = 1; j < len_b + 1; ++j) {
            int cost = a[i - 1] == b[j - 1] ? 0 : 1;

            d[i][j] = min(d[i-1][j] + 1, min(d[i][j-1] + 1, d[i-1][j-1] + cost));

            if ((i > 1) && (j > 1) && (a[i-1] == b[j-2]) && (a[i-2] == b[j-1])) {
                d[i][j] = min(d[i][j], d[i-2][j-2] + 1);
            }
        }
    }

    return d[len_a][len_b];
}
