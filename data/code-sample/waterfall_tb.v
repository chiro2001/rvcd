`timescale 1ns / 1ps

module tb_waterfall ();

  reg clk;
  reg rst;
  reg start;
  reg [1:0] freq_set;
  wire [7:0] led;

  waterfall u_waterfall (
      .clk(clk),
      .rst(rst),
      .start(start),
      .freq_set(freq_set),
      .led(led)
  );

  always #5 clk = ~clk;

  initial begin
    $dumpfile("tb.vcd");
    $dumpvars(0, clk, rst, start, freq_set, led);

    clk = 1'b0;

    #0 rst = 1'b0;
    start = 1'b0;
    freq_set = 2'b00;

    // test for 1 cycles, #10 * 16 == 160
    #10 rst = 1'b1;
    start = 1'b0;
    freq_set = 2'b00;
    #5 rst = 1'b0;
    start = 1'b1;
    #5 start = 1'b0;
    #160;

    // test for 2 cycles, #20 * 16 == 320
    #10 rst = 1'b1;
    start = 1'b0;
    freq_set = 2'b01;
    #5 rst = 1'b0;
    start = 1'b1;
    #5 start = 1'b0;
    #320;

    // test for 5 cycles, #50 * 8 = 400
    #10 rst = 1'b1;
    start = 1'b0;
    freq_set = 2'b10;
    #5 rst = 1'b0;
    start = 1'b1;
    #5 start = 1'b0;
    #400;

    // test for 10 cycles, #100 * 8 = 800
    #10 rst = 1'b1;
    start = 1'b0;
    freq_set = 2'b11;
    #5 rst = 1'b0;
    start = 1'b1;
    #5 start = 1'b0;
    #800;

    // for not start
    #10 rst = 1'b1;
    freq_set = 2'b01;
    #5 rst = 1'b0;
    #50 start = 1'b1;
    #5 start = 1'b0;
    #200;

    #10 $finish;
  end

endmodule

