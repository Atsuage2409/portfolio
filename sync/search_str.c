#include <stdio.h>

int main(void){
  FILE *f, *p;
  int c, k;
  f = fopen("logs/sync_list.log", "r");
  p = fopen("logs/make_link_list", "w");
  while((c = getc(f)) != EOF){
    if(c == '>'){
      k = 0;
      while(1){
	c = getc(f);
	if(c == ' '){
	  putc('/',p);
	  k = 1;
	  continue;
	}
	else if(c == '\n'){
	  putc(c,p);
	  break;
	}
	if(k == 1)
	  putc(c,p);
      }
    }
  }
  fclose(p);
  fclose(f);
  return 0;
}
