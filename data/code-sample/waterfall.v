module waterfall (
    input wire clk,
    input wire rst,
    input start,
    input wire [1:0] freq_set,
    output reg [7:0] led
);

reg [31:0] clock_cnt_limit;
reg [31:0] clock_cnt;
reg has_started;
wire test;

always @(posedge clk or posedge rst) begin
    if (rst) begin
        led <= 8'h01;
        clock_cnt <= 32'h0000_0001;
        has_started <= 1'b0;

        case (freq_set)
            2'b00:  clock_cnt_limit <= 32'd1_0000000;   // 1 cycle
            2'b01:  clock_cnt_limit <= 32'd2_0000000;   // 2 cycle
            2'b10:  clock_cnt_limit <= 32'd5_0000000;   // 5 cycle
            2'b11:  clock_cnt_limit <= 32'd10_0000000;   // 10 cycle
            default: clock_cnt_limit <= 32'h0000_0000;
        endcase
    end else begin
        if (start) begin
            has_started <= 1'b1;
        end

        if (has_started | start) begin
            if (clock_cnt == clock_cnt_limit) begin
                clock_cnt <= 32'h0000_0001;  
                led <= led == 8'h80 ? 8'h01 : led << 8'h1;
            end else begin
                clock_cnt <= clock_cnt + 32'h1; 
                led <= led;
            end
        end 
    end
end

endmodule