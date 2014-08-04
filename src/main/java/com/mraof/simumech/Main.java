package com.mraof.simumech;

import java.io.BufferedReader;
import java.io.IOException;
import java.io.InputStreamReader;
import java.util.HashMap;

import com.mraof.simumech.markov.LineSaver;
import com.mraof.simumech.markov.MarkovChain;

public class Main 
{
	public static String clientName = "Simumech";
	public static String version = "0";

	public static String[] owners = {"Mraof","Mraoffle"};
	public static MarkovChain markovChain;
	public static boolean useCR = true;
	public static boolean running = true;
	public static HashMap<String, IChat> chats = new HashMap<String, IChat>();
	public static HashMap<String, ChatLoader> chatLoaders = new HashMap<String, ChatLoader>();
	private static BufferedReader bufferedReader;
	private static InputStreamReader inputStreamReader;

	public static String[] espeakLangs = {"mb-en1", "mb-us1", "mb-us2", "mb-us3", "en", "en-us", "en-sc", "en-n", "en-rp", "en-wm"};
	public static String[] voiceModifiers = {"", "+f1", "+f2", "+f3", "+f4", "+f5", "+m1", "+m2", "+m3", "+m4", "+m5", "+m6", "+m7", "+croak", "+whisper"};

	public static void main(String args[])
	{
		if(args.length >= 1 && args[0].equals("noCR"));
		useCR = false;
		markovChain = new MarkovChain();
		LineSaver lineSaver = new LineSaver(markovChain);
		Thread autosave = new Thread(lineSaver);
		autosave.start();

		try
		{
			ChatLoader chatLoader = new ChatLoader();
			chats.put("irc", (IChat) (Class.forName("com.mraof.simumech.irc.IRC", false, chatLoader)).newInstance());
			chatLoaders.put("irc", chatLoader);
			System.out.println("irc loaded correctly");
		}
		catch(InstantiationException | IllegalAccessException | ClassNotFoundException e)
		{
			e.printStackTrace();
		};

		System.out.println("Does it even get here?");

		/*try
		{
			System.out.println("Attempting to load skype");
			ChatLoader chatLoader = new ChatLoader();
			chats.put("skype", (IChat) (Class.forName("com.mraof.simumech.skype.SkypeBot", false, chatLoader)).newInstance());
			chatLoaders.put("skype", chatLoader);
			System.out.println("skype loaded correctly");
		}
		catch(InstantiationException | IllegalAccessException | ClassNotFoundException e)
		{
			e.printStackTrace();
		};*/

		try
		{
			ChatLoader chatLoader = new ChatLoader();
			chats.put("tumblr", (IChat) (Class.forName("com.mraof.simumech.tumblr.Tumblr", false, chatLoader)).newInstance());
			chatLoaders.put("tumblr", chatLoader);
			System.out.println("tumblr loaded correctly");
		}
		catch(InstantiationException | IllegalAccessException | ClassNotFoundException e)
		{
			e.printStackTrace();
		};

		try
		{
			ChatLoader chatLoader = new ChatLoader();
			chats.put("twitter", (IChat) (Class.forName("com.mraof.simumech.twitter.TwitterChat", false, chatLoader)).newInstance());
			chatLoaders.put("twitter", chatLoader);
			System.out.println("twitter loaded correctly");
		}
		catch(InstantiationException | IllegalAccessException | ClassNotFoundException e)
		{
			e.printStackTrace();
		};



		inputStreamReader = new InputStreamReader(System.in);
		bufferedReader = new BufferedReader(inputStreamReader);
		String inputString;
		if(Profiler.instrumentation != null)
		{
			System.out.println("markovChain: " + Profiler.deepSizeOf(markovChain));
			System.out.println("chats: " + Profiler.deepSizeOf(chats));
		}

		try {
			while(running)
			{
				if(bufferedReader.ready())
				{
					if((inputString = bufferedReader.readLine()) != null)
						System.out.println(globalCommand(inputString));
				}
				else
				{
					try
					{
						Thread.sleep(200);
					} catch (InterruptedException e) {e.printStackTrace();}
				}
				//System.out.println(markovChain.reply(inputString));
			}
		} catch (IOException e) {
			e.printStackTrace();
		}
		running = false;
		System.out.println("Input loop done");

		for(IChat chat : chats.values())
			chat.quit();
		autosave.interrupt();

		try {
			Thread.sleep(5000);
		} catch (InterruptedException e) {e.printStackTrace();}


	}

	public static String globalCommand(String inputString) 
	{
		String result = "";
		String[] splitString = Util.splitFirst(inputString);
		System.out.println("Global command recieved: " + inputString);
		System.out.println(chats);
		try
		{
			if(inputString.equalsIgnoreCase("QUIT"))
			{
				running = false;
				//System.in.notifyAll();
			}
			String firstWord = splitString[0].toLowerCase();
			inputString = splitString[1];
			if(chats.containsKey(firstWord))
				return chats.get(firstWord).command(inputString);
			if(firstWord.equalsIgnoreCase("MARKOV"))
				return markovChain.command(inputString);
			else if(firstWord.equals("ts"))
			{
				try
				{
					String language = espeakLangs[(int) (Math.random() * espeakLangs.length)];
					String modifier = voiceModifiers[(int) (Math.random() * voiceModifiers.length)];
					result = markovChain.randomSentence();

					ProcessBuilder builder = new ProcessBuilder(new String[] {"espeak", "-v" + language + modifier, result});
					builder.start();
					result = language + modifier + ": " + result; 

				} catch(Exception e) 
				{
					return "Not supported";
				}
			}

			if(firstWord.equals("reload"))
			{
				IChat chat = chats.get(inputString);	
				if(chat != null)
				{
					chat.quit();
					chatLoaders.remove(inputString);
					ChatLoader chatLoader = new ChatLoader();				
					System.out.println("Reloading " + inputString + " (" +chat.getClass().getName() + ")");
					result = "Reloading " + firstWord + "failed";
					try
					{
						chats.put(firstWord, (IChat) (Class.forName(chat.getClass().getName(), false, chatLoader)).newInstance());
						result = "Reloaded" + firstWord;
					} catch (InstantiationException e) {
						e.printStackTrace();
					} catch (IllegalAccessException e) {
						e.printStackTrace();
					} catch (ClassNotFoundException e) {
						e.printStackTrace();
					}
					System.out.println("Done");
				}
			}
			else if(firstWord.equals("load"))
			{
				firstWord = splitString[0].toLowerCase();
				inputString = splitString[1];
				try
				{
					ChatLoader chatLoader = new ChatLoader();
					chats.put(firstWord, (IChat) (Class.forName(inputString, false, chatLoader)).newInstance());
					chatLoaders.put(firstWord, chatLoader);
					System.out.println("Loaded " + firstWord);
				}
				catch(InstantiationException | IllegalAccessException | ClassNotFoundException e)
				{
					e.printStackTrace();
				};

			}
			else if(firstWord.equals("unload"))
			{
				IChat chat = chats.get(inputString);
				if(chat != null)
				{
					chat.quit();
					chatLoaders.remove(inputString);
				}
			}
			return result;
		}
		catch(Exception e)
		{
			e.printStackTrace();
			return markovChain.reply(e.getMessage());
		}
	}
	public static String userCommand(String inputString)
	{
		int splitIndex = inputString.indexOf(' ');
		if(splitIndex != -1)
		{
			String firstWord = inputString.substring(0, splitIndex).toLowerCase();
			inputString = inputString.substring(splitIndex + 1);
			switch(firstWord.toUpperCase())
			{
				case "CALCULATE":
					try
					{
						//Process genius = Runtime.getRuntime().exec(new String[] {"genius", "--exec=" + inputString});
						ProcessBuilder builder = new ProcessBuilder(new String[] {"genius", "--exec=" + inputString});
						Process genius = builder.start();
						BufferedReader reader = new BufferedReader(new InputStreamReader(genius.getInputStream()));
						String answer = reader.readLine();
						reader.close();
						if(answer.length() > 500)
							answer = "Answer too long";
						return answer.isEmpty() ? "No answer" : answer;

					} catch(Exception e) 
					{
						return "Not supported";
					}
				case "M":
					return markovChain.command(inputString);

			}
		}
		return "";
	}
}
