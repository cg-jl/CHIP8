; use q and w keys to decrease/increase a sum.


draw_number: ; (n, x, y)
  LD VE, V1
  LD VD, V2
  BCD V0
  LDR V2
  FNT V0
  DRW VE, VD, 5
  ADD VE, 5
  FNT V1
  DRW VE, VD, 5
  ADD VE, 5
  FNT V2
  DRW VE, VD, 5 
  ADD VE, 5
  RET


main:
  ; n = 0 
  LD V3, 0
  ; while (true)
loop:
  ; if (is_pressed('q'))
check_q:
  LD V0, 4
  SIK V0
  JP check_w 
  ; n++;
  ADD V3, 1
  JP end_loop

check_w:
  ; else if (is_pressed('w'))
  LD V0, 5
  SIK V0
  JP loop ; else continue;
  ; n--;
  ADD V3, 0xff

end_loop:
  LD V0, V3
  LD V1, 32 - 8
  LD V2, 16 - 2 
  CALL draw_number
  JP loop



.entrypoint main
