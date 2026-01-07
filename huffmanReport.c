#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef struct _node{
  struct _node * l;
  struct _node * r;
  char c;
} node;

typedef struct {
  char c;
  float p;
  node *n;
} symbol;

void sort(symbol a[], int h, int t)
{
    int i, j;
    symbol temp;
    for (i = h; i < t; i++) {
        for (j = h; j < t - (i - h); j++) {
            if (a[j].p > a[j + 1].p) {
                temp = a[j];
                a[j] = a[j + 1];
                a[j + 1] = temp;
            }
        }
    }
}
// 木構造の表示
void printNode(node *n, int depth) {
    if (n == NULL) return;
    if (n->l == NULL && n->r == NULL) {
        printf("Leaf: %c (depth %d)\n", n->c, depth);
    } else {
        printNode(n->l, depth + 1);
        printNode(n->r, depth + 1);
    }
}

void huffmanTree(symbol a[],int n)
{
    int i;
    for (i = 0; i < n - 1; i++) {
        sort(a, i, n - 1);

        if (a[i].n == NULL) {
            a[i].n = (node*)malloc(sizeof(node));
            a[i].n->c = a[i].c;
            a[i].n->l = NULL;
            a[i].n->r = NULL;
        }

        if (a[i+1].n == NULL) {
            a[i+1].n = (node*)malloc(sizeof(node));
            a[i+1].n->c = a[i+1].c;
            a[i+1].n->l = NULL;
            a[i+1].n->r = NULL;
        }

        node *parent = (node*)malloc(sizeof(node));
        parent->c = '*';
        parent->l = a[i].n;
        parent->r = a[i+1].n;

        a[i+1].p = a[i].p + a[i+1].p;
        
        a[i+1].n = parent;

    }
}

void print_huffman_codes(node *root, char *code, int depth) {
    if (root == NULL) return;

    // 左の子へ
    if (root->l) {
        code[depth] = '0';
        print_huffman_codes(root->l, code, depth + 1);
    }

    // 右の子へ
    if (root->r) {
        code[depth] = '1';
        print_huffman_codes(root->r, code, depth + 1);
    }

    // 葉の場合
    if (root->l == NULL && root->r == NULL) {
        code[depth] = '\0';
        printf("Char: %c, Code: %s\n", root->c, code);
    }
}

#define N 4
int main(void)
{
  int i;
//   symbol ary[5]={{'a',0.1,NULL},{'b',0.15,NULL},{'c',0.3,NULL},{'d',0.4,NULL},{'e',0.05,NULL}};
     symbol ary[4]={{'a',0.1,NULL},{'b',0.2,NULL},{'c',0.3,NULL},{'d',0.4,NULL}};
  //  symbol ary[N]={{'a',0.1,NULL},{'b',0.9,NULL}};
  //  symbol ary[N]={{'a',0.1,NULL},{'b',0.2,NULL},{'c',0.7,NULL}};
  sort(ary,0,N-1);
  huffmanTree(ary,N);
  char code_buffer[100];
  printf("--- Huffman Codes ---\n");
  print_huffman_codes(ary[N-1].n, code_buffer, 0);
  return 0;
}