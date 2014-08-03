package com.mraof.simumech;

public interface IChat
{
	public void message(String message);
	public String command(String message);
	public void quit();
}
